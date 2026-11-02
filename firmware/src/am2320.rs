use embassy_time::{Duration, Timer};
use esp_hal::{Blocking, i2c::master::I2c};

const AM2320_ADDR: u8 = 0x5C; // 7-bit address (0xB8 >> 1)

#[derive(Debug)]
pub struct Measurement {
    pub humidity_x10: u16,
    pub temperature_x10: i16,
}

#[derive(Debug)]
pub enum Error {
    I2c(esp_hal::i2c::master::Error),
    BadResponse,
    Crc,
}

pub struct Am2320<'d> {
    i2c: I2c<'d, Blocking>,
}

impl<'d> Am2320<'d> {
    pub fn new(i2c: I2c<'d, Blocking>) -> Self {
        Self { i2c }
    }

    pub async fn read(&mut self) -> Result<Measurement, Error> {
        let _ = self.i2c.write(AM2320_ADDR, &[]);

        Timer::after(Duration::from_millis(1)).await;

        if let Err(e) = self.i2c.write(AM2320_ADDR, &[0x03, 0x00, 0x04]) {
            esp_println::println!("Command Error: {:?}", e);
        }

        Timer::after(Duration::from_millis(2)).await;

        let mut buf = [0u8; 8];
        if let Err(e) = self.i2c.read(AM2320_ADDR, &mut buf) {
            esp_println::println!("Read Error: {:?}", e);
            return Err(Error::I2c(e));
        }

        decode(&buf)
    }
}

fn decode(buf: &[u8;8]) -> Result<Measurement, Error> {
    let hum_high = buf[2] as u16;
    let hum_low = buf[3] as u16;

    let temp_high = buf[4] as u16;
    let temp_low = buf[5] as u16;

    let temp = temp_high * 256 + temp_low;
    let negative_temp = (temp & 0x8000) != 0;
    let temp = if negative_temp { -((temp & 0x7FFF) as i16) } else { (temp & 0x7FFF) as i16 };

    Ok(Measurement {
        humidity_x10: (hum_high * 256 + hum_low),
        temperature_x10: temp,
    })
}

fn crc16(data: &[u8]) -> u16 {
    todo!()
}
