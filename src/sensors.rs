use core::fmt::Debug;

use defmt::Debug2Format;
use fugit::ExtU64 as _;
use embedded_hal::i2c::I2c;
use crate::utils::AppTimer;

#[derive(Debug, Clone, Copy)]
pub struct SensorData {
    pub tmpr: f32,
    pub humi: f32,
}

pub async fn sht40_sensor_data<I2C, E: Debug, T: AppTimer>(i2c: &mut I2C, addr: u8) -> Result<SensorData, Sht40Error<E>>
where
    I2C: I2c<Error = E>,
{
    Sht40Command::MeasureHighRepeatability.send(i2c, addr)?;
    let data = sht40_read_data_with_retry::<6, _, _, T>(i2c, addr).await?;
    let tmpr = -45.0 + 175.0 * f32::from(u16::from_be_bytes([data[0], data[1]]) as f32 / 65535.0);
    let humi = 100.0 * f32::from(u16::from_be_bytes([data[3], data[4]]) as f32 / 65535.0);
    Ok(SensorData { tmpr, humi })
}

pub async fn sht40_read_data_with_retry<const SZ: usize, I2C, E: Debug, T: AppTimer>(i2c: &mut I2C, addr: u8) -> Result<[u8; SZ], Sht40Error<E>>
where
    I2C: I2c<Error = E>,
{
    for i in 1..=5 {
        match sht40_read_data(i2c, addr) {
            Ok(data) => return Ok(data),
            Err(Sht40Error::I2cError(e)) => {
                defmt::info!("retrying... {}/5 for error: {:?}", i, Debug2Format(&e));
                T::delay(5.millis()).await
            },
            Err(e) => return Err(e),
        }
    }
    Err(Sht40Error::TimeoutError)
}

#[derive(Debug)]
pub enum Sht40Error<E: Debug> {
    CrcError,
    I2cError(E),
    TimeoutError,
}

impl<E: Debug> From<E> for Sht40Error<E> {
    fn from(err: E) -> Self {
        Sht40Error::I2cError(err)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Sht40Command {
    MeasureHighRepeatability,
    MeasureMediumRepeatability,
    MeasureLowRepeatability,
    ReadSerial,
    SoftReset,
}

impl Sht40Command {
    pub fn to_byte(&self) -> u8 {
        match self {
            Sht40Command::MeasureHighRepeatability => 0xFD,
            Sht40Command::MeasureMediumRepeatability => 0xF6,
            Sht40Command::MeasureLowRepeatability => 0xE0,
            Sht40Command::ReadSerial => 0x89,
            Sht40Command::SoftReset => 0x94,
        }
    }

    /// Well, the sensor is slow, if you do write_read for sensor data retrieving,
    /// you will definitely get a NACK in return. Instead, do write cmd and read separately.
    pub fn send<I2C, E: Debug>(&self, i2c: &mut I2C, addr: u8) -> Result<(), Sht40Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        i2c.write(addr, &[self.to_byte()])?;
        Ok(())
    }
}

pub fn sht40_read_data<const SZ: usize, I2C, E: Debug>(i2c: &mut I2C, addr: u8) -> Result<[u8; SZ], Sht40Error<E>>
where
    I2C: I2c<Error = E>,
{
    let mut out = [0; SZ];
    i2c.read(addr, &mut out)?;
    sht40_verify_crc(&out)?;
    Ok(out)
}

fn sht40_verify_crc<E: Debug>(data: &[u8]) -> Result<(), Sht40Error<E>> {
    for i in 0..data.len() / 3 {
        if crc8(&data[i*3..i*3+2]) != data[i*3+2] {
            return Err(Sht40Error::CrcError);
        }
    }
    Ok(())
}

fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0xff;
    for byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x31;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
