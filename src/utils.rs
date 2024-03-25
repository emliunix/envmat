use display_interface::DisplayError;
use embedded_hal::i2c::ErrorType;
use embedded_hal_02::blocking::i2c::Write;
use embedded_hal::i2c::I2c;
use rtic::Mutex;

use crate::rp2040_monotonics;

pub struct I2CWrapper<F> where
{
    inner: F
}

impl<I2C, M> I2CWrapper<M> where
    M: Mutex<T=I2C>
{
    pub fn new(i2c: M) -> Self {
        I2CWrapper {
            inner: i2c,
        }
    }
}

impl<I2C, M> Write for I2CWrapper<M> where
    M: Mutex<T=I2C>,
    I2C: Write,
{
    type Error = I2C::Error;

    fn write(&mut self, addr: u8, output: &[u8]) -> Result<(), Self::Error> {
        // log addr and output
        // defmt::info!("addr: {:?}, output: {:?}", addr, defmt::Debug2Format(output));
        self.inner.lock(|i2c| i2c.write(addr, output))
    }
}

impl<I2C: I2c, M: Mutex<T=I2C>> ErrorType for I2CWrapper<M> {
    type Error = <I2C as ErrorType>::Error;
}

impl<I2C, M> I2c for I2CWrapper<M> where
    M: Mutex<T=I2C>,
    I2C: I2c,
{
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.inner.lock(|i2c| {
            i2c.transaction(address, operations)
        })
    }
}

type Duration = fugit::TimerDurationU64<1_000_000>;

pub trait AppTimer {
    async fn delay(duration: Duration);
}

pub struct Rp2040Timer();

impl AppTimer for Rp2040Timer {
    #[inline]
    async fn delay(duration: Duration) {
        rp2040_monotonics::Timer::delay(duration).await;
    }
}

pub fn log_display_error<T>(res: Result<T, DisplayError>) {
    if let Err(e) = res {
        defmt::error!("error: {:?}", defmt::Debug2Format(&e));
    }
}
