use esp_idf_hal::delay::TickType;
use esp_idf_hal::i2c::I2cDriver;
use esp_idf_sys::EspError;

const GT911_I2C_ADDR_PRIMARY: u8 = 0x5D;
const GT911_I2C_ADDR_ALT: u8 = 0x14;
const GT911_REG_STATUS: u16 = 0x814E;
const GT911_REG_POINT1: u16 = 0x8150;
const I2C_TIMEOUT_MS: u64 = 100;

#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum TouchEvent {
    Touch(TouchPoint),
    Slide { from: TouchPoint, to: TouchPoint },
}

#[derive(Debug, Default)]
pub struct TouchTracker {
    last_point: Option<TouchPoint>,
    min_delta: u16,
}

impl TouchTracker {
    pub fn new(min_delta: u16) -> Self {
        Self {
            last_point: None,
            min_delta,
        }
    }

    pub fn on_sample(&mut self, point: Option<TouchPoint>) -> Option<TouchEvent> {
        match (self.last_point, point) {
            (None, Some(current)) => {
                self.last_point = Some(current);
                Some(TouchEvent::Touch(current))
            }
            (Some(previous), Some(current)) => {
                if !self.moved_enough(previous, current) {
                    return None;
                }
                self.last_point = Some(current);
                Some(TouchEvent::Slide {
                    from: previous,
                    to: current,
                })
            }
            (_, None) => {
                self.last_point = None;
                None
            }
        }
    }

    fn moved_enough(&self, from: TouchPoint, to: TouchPoint) -> bool {
        let dx = from.x.abs_diff(to.x);
        let dy = from.y.abs_diff(to.y);
        dx >= self.min_delta || dy >= self.min_delta
    }
}

pub struct Gt911<'d> {
    i2c: I2cDriver<'d>,
    address: u8,
}

impl<'d> Gt911<'d> {
    pub fn new(i2c: I2cDriver<'d>, address: u8) -> Self {
        Self { i2c, address }
    }

    pub fn detect_address(i2c: &mut I2cDriver<'d>) -> Option<u8> {
        if Self::probe_address(i2c, GT911_I2C_ADDR_PRIMARY) {
            return Some(GT911_I2C_ADDR_PRIMARY);
        }

        if Self::probe_address(i2c, GT911_I2C_ADDR_ALT) {
            return Some(GT911_I2C_ADDR_ALT);
        }

        None
    }

    pub fn read_touch(&mut self) -> Result<Option<TouchPoint>, EspError> {
        let status = self.read_u8(GT911_REG_STATUS)?;
        if status & 0x80 == 0 {
            return Ok(None);
        }

        let points = status & 0x0F;
        if points == 0 {
            self.write_u8(GT911_REG_STATUS, 0x00)?;
            return Ok(None);
        }

        let mut data = [0u8; 4];
        self.read_bytes(GT911_REG_POINT1, &mut data)?;
        self.write_u8(GT911_REG_STATUS, 0x00)?;

        Ok(Some(TouchPoint {
            x: u16::from_le_bytes([data[0], data[1]]),
            y: u16::from_le_bytes([data[2], data[3]]),
        }))
    }

    fn read_u8(&mut self, register: u16) -> Result<u8, EspError> {
        let mut value = [0u8; 1];
        self.read_bytes(register, &mut value)?;
        Ok(value[0])
    }

    fn write_u8(&mut self, register: u16, value: u8) -> Result<(), EspError> {
        let buffer = [Self::reg_high(register), Self::reg_low(register), value];
        let timeout = TickType::new_millis(I2C_TIMEOUT_MS).ticks();
        self.i2c.write(self.address, &buffer, timeout)
    }

    fn read_bytes(&mut self, register: u16, buffer: &mut [u8]) -> Result<(), EspError> {
        let register_bytes = [Self::reg_high(register), Self::reg_low(register)];
        let timeout = TickType::new_millis(I2C_TIMEOUT_MS).ticks();
        self.i2c
            .write_read(self.address, &register_bytes, buffer, timeout)
    }

    fn probe_address(i2c: &mut I2cDriver<'d>, address: u8) -> bool {
        let register_bytes = [
            Self::reg_high(GT911_REG_STATUS),
            Self::reg_low(GT911_REG_STATUS),
        ];
        let mut value = [0u8; 1];
        let timeout = TickType::new_millis(I2C_TIMEOUT_MS).ticks();
        i2c.write_read(address, &register_bytes, &mut value, timeout)
            .is_ok()
    }

    fn reg_high(register: u16) -> u8 {
        (register >> 8) as u8
    }

    fn reg_low(register: u16) -> u8 {
        (register & 0xFF) as u8
    }
}
