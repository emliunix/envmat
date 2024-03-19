#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use seeeduino_xiao_rp2040 as bsp;
use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [TIMER_IRQ_0, SW0_IRQ])]
mod app {
    use fugit::ExtU64;
    use embedded_hal::digital::v2::OutputPin;
    use rtic_monotonics::rp2040;
    use seeeduino_xiao_rp2040 as bsp;
    use bsp::hal::{
        clocks::{init_clocks_and_plls, Clock}, gpio::{Pin, PushPullOutput}, pac, sio::Sio, watchdog::Watchdog
    };


    #[shared]
    struct Shared {
    }

    #[local]
    struct Local {
        led_green: Pin<bsp::hal::gpio::bank0::Gpio16, bsp::hal::gpio::PushPullOutput>
    }


    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        defmt::info!("Hello, world!");
        let mut pac = cx.device;
        let mut watchdog = Watchdog::new(pac.WATCHDOG);
        let sio = Sio::new(pac.SIO);

        defmt::info!("initializing clocks...");
        let clocks = init_clocks_and_plls(
            bsp::XOSC_CRYSTAL_FREQ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        ).ok().unwrap();

        defmt::info!("initializing pins...");
        let pins = bsp::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );
        
        defmt::info!("initializing timer...");
        rp2040::Timer::start(pac.TIMER, &mut pac.RESETS, rtic_monotonics::create_rp2040_monotonic_token!());

        defmt::info!("initializing led...");
        let led_green = pins.led_green.into_push_pull_output();

        defmt::info!("spawning tasks...");
        led::spawn().unwrap();
        
        defmt::info!("intialization complete!");
        (Shared { }, Local { led_green })
    }
    
    #[task(local = [led_green])]
    async fn led(cx: led::Context) {
        defmt::info!("blink led!");

        let led_green = cx.local.led_green;

        loop {
            defmt::info!("on!");
            led_green.set_high().unwrap();
            rp2040::Timer::delay(500u64.millis()).await;
            defmt::info!("off!");
            led_green.set_low().unwrap();
            rp2040::Timer::delay(500u64.millis()).await;
        }
    }
}
