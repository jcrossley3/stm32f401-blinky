#![no_main]
#![no_std]

// Network-specfic values
const WIFI_SSID: &str = include_str!("wifi.ssid.txt");
const WIFI_PASSWORD: &str = include_str!("wifi.password.txt");
const HOST: &str = "192.168.0.110";
const HOST_HEADER: &str = "http-endpoint.drogue-iot.10.104.208.20.nip.io";

mod device;

use core::str::from_utf8;
use core::str::FromStr;
use heapless::consts::{U512, U1024};

use log::{info, LevelFilter};
use rtt_logger::RTTLogger;
use rtt_target::rtt_init_print;
use panic_rtt_target as _;

use rtic::app;
use rtic::cyccnt::U32Ext;

use drogue_http_client::{tcp::TcpSocketSinkSource, BufferResponseHandler, HttpConnection, Source};
use drogue_esp8266::{ingress::Ingress, protocol::WiFiMode};
use drogue_network::{
    tcp::{Mode, TcpStack},
    addr::{HostSocketAddr, HostAddr},
};

static LOGGER: RTTLogger = RTTLogger::new(LevelFilter::Debug);
const DIGEST_DELAY: u32 = 200;

#[app(device = nucleo_f401re::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        adapter: Option<device::ESPAdapter>,
        ingress: Ingress<'static, device::SerialRx>,
    }

    #[init(spawn = [digest])]
    fn init(ctx: init::Context) -> init::LateResources {
        rtt_init_print!( BlockIfFull, 2048);
        log::set_logger(&LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Trace);

        // Enable CYCNT
        let mut cmp = ctx.core;
        cmp.DWT.enable_cycle_counter();

        let (adapter, ingress) = device::network_adapter(ctx.device);

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
	    HostAddr::from_str(HOST).unwrap(),
            8080,
        );

        let mut socket = network.connect(socket, socket_addr).unwrap();

        info!("socket connected {:?}", result);

	let mut tcp = TcpSocketSinkSource::from(&mut network, &mut socket);

	let con = HttpConnection::<U1024>::new();

	// dummy test data
	let data = r#"{"temp": 41.23}"#;

	let handler = BufferResponseHandler::<U1024>::new();

	log::info!("Starting request...");

	let mut req = con
            .post("/publish/device_id/foo")
            .headers(&[("Host", HOST_HEADER),
		       ("Content-Type", "text/json")])
            .handler(handler)
            .execute_with::<_, U512>(&mut tcp, Some(data.as_bytes()));

	log::info!("Request sent, piping data...");

	tcp.pipe_data(&mut req).unwrap();

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
