#![no_std]
#![no_main]

mod xiao_rp2040;
mod rp2040_monotonics;
mod display;
mod utils;

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = crate::xiao_rp2040::pac, peripherals = true, dispatchers = [SW0_IRQ])]
mod app {
    use fugit::{ExtU64, RateExtU32};
    use embedded_hal_02::digital::v2::OutputPin;
    use rp2040_hal::gpio::PullUp;
    use crate::{rp2040_monotonics, xiao_rp2040 as bsp};
    use bsp::hal::{
        clocks::{init_clocks_and_plls, Clock}, sio::Sio, watchdog::Watchdog, I2C,
        gpio::FunctionI2C,
    };
    use bsp::pac;
    use ssd1306::{command::AddrMode, prelude::Brightness, rotation::DisplayRotation, size::DisplaySize128x64, I2CDisplayInterface, Ssd1306};

    use crate::{
        display::draw,
        utils::{I2CWrapper, log_display_error}
    };

    #[shared]
    struct Shared {
        i2c: I2C<pac::I2C1, (bsp::Sda, bsp::Scl)>,
    }

    #[local]
    struct Local {
        led_green: bsp::LedGreen,
        led_red: bsp::LedRed,
        led_blue: bsp::LedBlue,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
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
        rp2040_monotonics::Timer::start(pac.TIMER, &pac.RESETS);
        
        defmt::info!("initializing led...");
        let led_green = pins.led_green.into_push_pull_output();
        let led_red = pins.led_red.into_push_pull_output();
        let led_blue = pins.led_blue.into_push_pull_output();

        //let (s, r) = make_channel!(f32, CHANNEL_SIZE);
        defmt::info!("spawning tasks...");
        if let Err(err) = led::spawn() {
            defmt::error!("failed to spawn led task: {:?}", err);
        }
        if let Err(err) = display::spawn() {
            defmt::error!("failed to spawn display task: {:?}", err);
        }

        let i2c = I2C::i2c1(
            pac.I2C1,
            pins.sda.into_pull_type::<PullUp>().into_function::<FunctionI2C>(),
            pins.scl.into_pull_type::<PullUp>().into_function::<FunctionI2C>(),
            100u32.kHz(),
            &mut pac.RESETS,
            clocks.system_clock.freq());

        defmt::info!("intialization complete!");
        (Shared { i2c }, Local { led_green, led_red, led_blue })
    }
    
    #[task(local = [led_green, led_red, led_blue])]
    async fn led(cx: led::Context) {
        defmt::info!("blink led!");

        let led_green = cx.local.led_green;
        cx.local.led_red.set_high().unwrap();
        cx.local.led_blue.set_high().unwrap();

        loop {
            defmt::info!("on!");
            led_green.set_high().unwrap();
            rp2040_monotonics::Timer::delay(500u64.millis()).await;
            defmt::info!("off!");
            led_green.set_low().unwrap();
            rp2040_monotonics::Timer::delay(500u64.millis()).await;
        }
    }

    #[task(shared = [i2c])]
    async fn display(cx: display::Context) {
        // plot text temperate: ${tmp} with embedded_graphics
        defmt::info!("display!");
        let i2cdi = I2CDisplayInterface::new(I2CWrapper::new(cx.shared.i2c));
        let mut ssd1306 = Ssd1306::new(i2cdi, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        log_display_error(ssd1306.init_with_addr_mode(AddrMode::Page));
        log_display_error(ssd1306.set_brightness(Brightness::DIM));
        loop {
            defmt::info!("draw!");
            draw(&mut ssd1306, 23.6);
            log_display_error(ssd1306.flush());
            rp2040_monotonics::Timer::delay(5000u64.millis()).await;
        }
    }
}
