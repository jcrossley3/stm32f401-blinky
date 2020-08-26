#![no_main]
#![no_std]

#[allow(unused)]
use panic_halt;

extern crate stm32f4xx_hal;
use stm32f4xx_hal::{delay::Delay, prelude::*, stm32};

use cortex_m::peripheral::Peripherals;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    if let (Some(mut p), Some(cp)) = (stm32::Peripherals::take(), Peripherals::take()) {
        cortex_m::interrupt::free(move |cs| {
            // Configure clock to 8 MHz (i.e. the default) and freeze it
            let mut rcc = p.RCC.constrain().cfgr.sysclk(8.mhz()).freeze();

            // (Re-)configure PA5 as output
            let gpioa = p.GPIOA.split();
            let mut led = gpioa.pa5.into_push_pull_output();

            // Get delay provider
            let mut delay = Delay::new(cp.SYST, rcc);

            // Toggle the LED roughly every second
            loop {
                led.toggle();
                delay.delay_ms(1_000_u16);
            }
        });
    }

    loop {
        continue;
    }
}
