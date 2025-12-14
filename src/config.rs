#[derive(Debug, Clone, Copy)]
pub enum LcdCharsize {
    DOTS5x10,
    DOTS5x8,
}

impl From<LcdCharsize> for u8 {
    fn from(val: LcdCharsize) -> u8 {
        match val {
            LcdCharsize::DOTS5x10 => 0x04,
            LcdCharsize::DOTS5x8 => 0x00,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LcdBacklight {
    Backlight,
    NoBacklight,
}

impl From<LcdBacklight> for u8 {
    fn from(value: LcdBacklight) -> Self {
        match value {
            LcdBacklight::Backlight => 0x08,
            LcdBacklight::NoBacklight => 0x00,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: u8,
    pub rows: u8,
    pub charsize: LcdCharsize,
    pub lcd_backlight: LcdBacklight,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: 0x3f,
            rows: 2,
            charsize: LcdCharsize::DOTS5x8,
            lcd_backlight: LcdBacklight::Backlight,
        }
    }
}
