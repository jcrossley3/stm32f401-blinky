#![no_main]
#![no_std]

use nucleo_f401re::{
    pac::USART6,
    hal::{
        prelude::*,
        serial::{
            config::{Config, Parity, StopBits},
            Serial,
            Tx,
            Rx,
        },
    },
};
use core::str::from_utf8;
use rtt_logger::RTTLogger;
use log::{info, LevelFilter};
use rtt_target::rtt_init_print;
use panic_rtt_target as _;

use heapless::{
    spsc::Queue,
    i,
    consts::{
        U2,
        U16,
	U512,
	U1024,
    },
};

use drogue_http_client::{tcp::TcpSocketSinkSource, BufferResponseHandler, HttpConnection, Source};

use drogue_esp8266::{
    initialize,
    ingress::Ingress,
    adapter::Adapter,
    protocol::Response,
    protocol::WiFiMode,
};
use drogue_network::{
    tcp::{
        Mode,
        TcpStack,
    },
    addr::{
        HostSocketAddr,
        HostAddr,
    },
};
use rtic::app;
use rtic::cyccnt::U32Ext;

const WIFI_SSID: &'static str = include_str!("wifi.ssid.txt");
const WIFI_PASSWORD: &'static str = include_str!("wifi.password.txt");

static LOGGER: RTTLogger = RTTLogger::new(LevelFilter::Debug);
const DIGEST_DELAY: u32 = 200;

type SerialTx = Tx<USART6>;
type SerialRx = Rx<USART6>;
type ESPAdapter = Adapter<'static, SerialTx>;

#[app(device = nucleo_f401re::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        adapter: Option<ESPAdapter>,
        ingress: Ingress<'static, SerialRx>,
    }

    #[init(spawn = [digest])]
    fn init(ctx: init::Context) -> init::LateResources {
        rtt_init_print!( BlockIfFull, 2048);
        log::set_logger(&LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Trace);

        // Enable CYCNT
        let mut cmp = ctx.core;
        cmp.DWT.enable_cycle_counter();

        let device: nucleo_f401re::pac::Peripherals = ctx.device;

        let rcc = device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();

        let gpioa = device.GPIOA.split();
        let gpioc = device.GPIOC.split();

        let pa11 = gpioa.pa11;
        let pa12 = gpioa.pa12;

        // SERIAL pins for USART6
        let tx_pin = pa11.into_alternate_af8();
        let rx_pin = pa12.into_alternate_af8();

        // enable pin
        let mut en = gpioc.pc10.into_push_pull_output();
        // reset pin
        let mut reset = gpioc.pc12.into_push_pull_output();

        let usart6 = device.USART6;

        let mut serial = Serial::usart6(
            usart6,
            (tx_pin, rx_pin),
            Config {
                baudrate: 115_200.bps(),
                parity: Parity::ParityNone,
                stopbits: StopBits::STOP1,
                ..Default::default()
            },
            clocks,
        ).unwrap();

        serial.listen(nucleo_f401re::hal::serial::Event::Rxne);
        let (tx, rx) = serial.split();

        static mut RESPONSE_QUEUE: Queue<Response, U2> = Queue(i::Queue::new());
        static mut NOTIFICATION_QUEUE: Queue<Response, U16> = Queue(i::Queue::new());

        let (adapter, ingress) = initialize(
            tx, rx,
            &mut en, &mut reset,
            unsafe { &mut RESPONSE_QUEUE },
            unsafe { &mut NOTIFICATION_QUEUE },
        ).unwrap();

        ctx.spawn.digest().unwrap();

        info!("initialized");

        init::LateResources {
            adapter: Some(adapter),
            ingress,
        }
    }

    #[task(schedule = [digest], priority = 2, resources = [ingress])]
    fn digest(mut ctx: digest::Context) {
        ctx.resources.ingress.lock(|ingress| ingress.digest());
        ctx.schedule.digest(ctx.scheduled + (DIGEST_DELAY * 100_000).cycles())
            .unwrap();
    }

    #[task(binds = USART6, priority = 10, resources = [ingress])]
    fn usart(ctx: usart::Context) {
        if let Err(b) = ctx.resources.ingress.isr() {
            info!("failed to ingress {}", b as char);
        }
    }

    #[idle(resources = [adapter])]
    fn idle(ctx: idle::Context) -> ! {
        info!("idle");

        let mut adapter = ctx.resources.adapter.take().unwrap();

        let result = adapter.get_firmware_info();
        info!("firmware: {:?}", result);

        let result = adapter.set_mode(WiFiMode::Station);
        info!("set mode {:?}", result);

        let result = adapter.join(WIFI_SSID, WIFI_PASSWORD);
        info!("joined wifi {:?}", result);

        let result = adapter.get_ip_address();
        info!("IP {:?}", result);

        let mut network = adapter.into_network_stack();
        info!("network intialized");

        let socket = network.open(Mode::Blocking).unwrap();
        info!("socket {:?}", socket);

        let socket_addr = HostSocketAddr::new(
	    HostAddr::ipv4([192,168,0,110]),
            8080,
        );

        let mut socket = network.connect(socket, socket_addr).unwrap();

        info!("socket connected {:?}", result);

	let mut tcp = TcpSocketSinkSource::from(&mut network, &mut socket);

	let con = HttpConnection::<U1024>::new();

	let data = r#"{"temp": 41.23}"#;

	let handler = BufferResponseHandler::<U1024>::new();

	log::info!("Starting request...");

	let mut req = con
            .post("/publish/device_id/foo")
            .headers(&[("Host", "http-endpoint.drogue-iot.10.109.177.179.nip.io"), ("Content-Type", "text/json")])
            .handler(handler)
            .execute_with::<_, U512>(&mut tcp, Some(data.as_bytes()));

	log::info!("Request sent, piping data...");

	tcp.pipe_data(&mut req);

	log::info!("Done piping data, checking result");

	let (_, handler) = req.complete();

	log::info!(
            "Result: {} {}, Payload: {:?}",
            handler.code(),
            handler.reason(),
            from_utf8(handler.payload())
	);

        loop {
            continue;
        }

    }

    // spare interrupt used for scheduling software tasks
    extern "C" {
        fn SPI1();
        fn SPI2();
    }
};
