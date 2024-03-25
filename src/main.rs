#![no_std]
#![no_main]

mod xiao_rp2040;
mod rp2040_monotonics;
mod sensors;
mod display;
mod utils;

use defmt_rtt as _;
use panic_probe as _;

const SENSOR_DATA_CAP: usize = 2;

#[rtic::app(device = crate::xiao_rp2040::pac, peripherals = true, dispatchers = [TIMER_IRQ_0, SW0_IRQ])]
mod app {
    use defmt::Debug2Format;
    use fugit::{ExtU64, RateExtU32};
    use embedded_hal_02::digital::v2::OutputPin;
    use rp2040_hal::gpio::PullUp;
    use rtic_sync::{channel::{Receiver, Sender}, make_channel};
    use crate::{create_rp2040_monotonic_token, rp2040_monotonics, xiao_rp2040 as bsp};
    use bsp::hal::{
        clocks::{init_clocks_and_plls, Clock}, sio::Sio, watchdog::Watchdog, I2C,
        gpio::FunctionI2C,
    };
    use bsp::pac;
    use ssd1306::{mode::DisplayConfig as _, rotation::DisplayRotation, size::DisplaySize128x64, I2CDisplayInterface, Ssd1306};

    use crate::{
        sensors::{Sht40Command, SensorData, sht40_sensor_data, sht40_read_data_with_retry}, SENSOR_DATA_CAP,
        display::draw,
        utils::{I2CWrapper, log_display_error, Rp2040Timer, AppTimer as _}
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
        defmt::info!("initializing...!");
        let mut pac = cx.device;
        let mut watchdog = Watchdog::new(pac.WATCHDOG);
        let sio = Sio::new(pac.SIO);

        let clocks = init_clocks_and_plls(
            bsp::XOSC_CRYSTAL_FREQ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        ).ok().unwrap();

        let pins = bsp::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );
        
        create_rp2040_monotonic_token!();
        rp2040_monotonics::Timer::start(pac.TIMER, &pac.RESETS);
        
        let led_green = pins.led_green.into_push_pull_output();
        let led_red = pins.led_red.into_push_pull_output();
        let led_blue = pins.led_blue.into_push_pull_output();

        let (s, r) = make_channel!(SensorData, {SENSOR_DATA_CAP});
        if let Err(err) = led::spawn() {
            defmt::error!("failed to spawn led task: {:?}", err);
        }
        if let Err(err) = display::spawn(r) {
            defmt::error!("failed to spawn display task: {:?}", err);
        }
        if let Err(err) = sensors::spawn(s) {
            defmt::error!("failed to spawn sensors task: {:?}", err);
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
            // defmt::info!("on!");
            led_green.set_high().unwrap();
            rp2040_monotonics::Timer::delay(500u64.millis()).await;
            // defmt::info!("off!");
            led_green.set_low().unwrap();
            rp2040_monotonics::Timer::delay(500u64.millis()).await;
        }
    }

    #[task(shared = [i2c])]
    async fn display(cx: display::Context, mut receiver: Receiver<'static, SensorData, {SENSOR_DATA_CAP}>) {
        // plot text temperate: ${tmp} with embedded_graphics
        defmt::info!("display!");
        let i2cdi = I2CDisplayInterface::new(I2CWrapper::new(cx.shared.i2c));
        let mut ssd1306 = Ssd1306::new(i2cdi, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        // log_display_error(ssd1306.init_with_addr_mode(AddrMode::Page));
        log_display_error(ssd1306.init());
        // log_display_error(ssd1306.set_brightness(Brightness::DIM));
        loop {
            let data = receiver.recv().await.unwrap();
            draw(&mut ssd1306, data.tmpr, data.humi);
            log_display_error(ssd1306.flush());
            rp2040_monotonics::Timer::delay(5000u64.millis()).await;
        }
    }

    #[task(shared = [i2c])]
    async fn sensors(cx: sensors::Context, mut sender: Sender<'static, SensorData, {SENSOR_DATA_CAP}>) {
        let mut i2c = I2CWrapper::new(cx.shared.i2c);
        let sht40_addr: u8 = 0x44u8;
        Sht40Command::ReadSerial.send(&mut i2c, sht40_addr).unwrap();
        match sht40_read_data_with_retry::<6, _, _, Rp2040Timer>(&mut i2c, sht40_addr).await {
            Ok(serial) => {
                defmt::info!("SHT40 serial: 0x{:02x}{:02x}{:02x}{:02x}", &serial[0], &serial[1], &serial[3], &serial[5]);
            }
            Err(e) => {
                defmt::error!("error reading SHT40 serial: {:?}", defmt::Debug2Format(&e));
            }
        }

        loop {
            match sht40_sensor_data::<_, _, Rp2040Timer>(&mut i2c, sht40_addr).await {
                Ok(data) => {
                    defmt::info!("measured {:?}", Debug2Format(&data));
                    sender.send(data).await.unwrap();
                }
                Err(e) => {
                    defmt::error!("error reading SHT40 sensor data: {:?}", defmt::Debug2Format(&e));
                }
            }
            Rp2040Timer::delay(5000u64.millis()).await;
        }
    }
}
