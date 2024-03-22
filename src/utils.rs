use display_interface::DisplayError;
use embedded_hal_02::blocking::i2c::Write;
use rtic::Mutex;

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
        self.inner.lock(|i2c| i2c.write(addr, output))
    }
}

pub fn log_display_error<T>(res: Result<T, DisplayError>) {
    if let Err(e) = res {
        defmt::error!("error: {:?}", defmt::Debug2Format(&e));
    }
}
