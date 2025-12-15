use crate::config::{Config, LcdBacklight};
use crate::traits::{I2CAsync, SleepableAsync};

const EN: u8 = 0b00000100; // Enable bit
const RS: u8 = 0b00000001; // Register select bit

const LCD_CLEARDISPLAY: u8 = 0x01;
const LCD_RETURNHOME: u8 = 0x02;
const LCD_ENTRYMODESET: u8 = 0x04;
const LCD_DISPLAYCONTROL: u8 = 0x08;
const LCD_CURSORSHIFT: u8 = 0x10;
const LCD_FUNCTIONSET: u8 = 0x20;
const LCD_SETDDRAMADDR: u8 = 0x80;

// flags for display entry mode
const LCD_ENTRYLEFT: u8 = 0x02;
const LCD_ENTRYSHIFTINCREMENT: u8 = 0x01;
const LCD_ENTRYSHIFTDECREMENT: u8 = 0x00;

// flags for display on/off control
const LCD_DISPLAYON: u8 = 0x04;
const LCD_CURSORON: u8 = 0x02;
const LCD_CURSOROFF: u8 = 0x00;
const LCD_BLINKON: u8 = 0x01;
const LCD_BLINKOFF: u8 = 0x00;

// flags for display/cursor shift
const LCD_DISPLAYMOVE: u8 = 0x08;
const LCD_MOVERIGHT: u8 = 0x04;
const LCD_MOVELEFT: u8 = 0x00;

// flags for function set
// const LCD_8BITMODE: u8 = 0x10;
// const LCD_4BITMODE: u8 = 0x00;
const LCD_2LINE: u8 = 0x08;
const LCD_1LINE: u8 = 0x00;

pub struct Lcd<T, S>
where
    T: I2CAsync,
    S: SleepableAsync,
{
    // config: Config,
    i2c: T,
    sleep: S,

    addr: u8,
    rows: u8,
    backlight: LcdBacklight,
    display_function: u8,
    display_control: u8,
    display_mode: u8,
}

impl<T, S> Lcd<T, S>
where
    T: I2CAsync,
    S: SleepableAsync,
{
    pub fn new(i2c: T, sleep: S, config: Config) -> Self {
        let charsize: u8 = config.charsize.into();
        let linesize = if config.rows > 1 {
            LCD_2LINE
        } else {
            LCD_1LINE
        };

        let display_function: u8 = charsize | linesize;

        let display_control: u8 = LCD_DISPLAYON | LCD_CURSOROFF | LCD_BLINKOFF;

        let display_mode: u8 = LCD_ENTRYLEFT | LCD_ENTRYSHIFTDECREMENT;
        let backlight = config.lcd_backlight;

        Self {
            i2c,
            sleep,
            display_function,
            display_control,
            display_mode,
            backlight,
            rows: config.rows,
            addr: config.addr,
        }
    }

    async fn expander_write(&mut self, data: u8) -> Result<(), T::Error> {
        let light: u8 = self.backlight.into();
        self.i2c.write_async(self.addr, [light | data]).await
    }

    async fn pulse_enable(&mut self, data: u8) -> Result<(), T::Error> {
        self.expander_write(data | EN).await?;
        self.sleep.sleep_for_micros(1).await;
        self.expander_write(data | !EN).await?;
        self.sleep.sleep_for_micros(50).await;

        Ok(())
    }

    async fn write_4_bits(&mut self, data: u8) -> Result<(), T::Error> {
        self.expander_write(data).await?;
        self.pulse_enable(data).await?;
        Ok(())
    }

    async fn send(&mut self, data: u8, mode: u8) -> Result<(), T::Error> {
        self.write_4_bits(data & 0xf0 | mode).await?;
        self.write_4_bits((data << 4) & 0xf0 | mode).await?;
        Ok(())
    }

    async fn command(&mut self, data: u8) -> Result<(), T::Error> {
        self.send(data, 0).await?;
        Ok(())
    }

    async fn write(&mut self, data: u8) -> Result<(), T::Error> {
        self.send(data, RS).await?;
        Ok(())
    }

    pub async fn begin(mut self) -> Result<LcdInitialized<T, S>, T::Error> {
        self.sleep.sleep_for_millis(50).await;

        self.expander_write(self.backlight.into()).await?;
        self.sleep.sleep_for_millis(1000).await;

        self.write_4_bits(0x03 << 4).await?;
        self.sleep.sleep_for_micros(4500).await;

        self.write_4_bits(0x03 << 4).await?;
        self.sleep.sleep_for_micros(4500).await;

        self.write_4_bits(0x03 << 4).await?;
        self.sleep.sleep_for_micros(150).await;

        self.write_4_bits(0x02 << 4).await?;

        self.command(self.display_function | LCD_FUNCTIONSET)
            .await?;

        let mut lcd = LcdInitialized { lcd: self };

        lcd.display().await?;

        lcd.clear().await?;

        lcd.lcd
            .command(LCD_ENTRYMODESET | lcd.lcd.display_mode)
            .await?;

        lcd.home().await?;

        Ok(lcd)
    }
}

pub struct LcdInitialized<T, S>
where
    T: I2CAsync,
    S: SleepableAsync,
{
    lcd: Lcd<T, S>,
}

impl<T, S> LcdInitialized<T, S>
where
    T: I2CAsync,
    S: SleepableAsync,
{
    pub async fn display(&mut self) -> Result<(), T::Error> {
        self.lcd.display_control |= LCD_DISPLAYON;
        self.lcd
            .command(LCD_DISPLAYCONTROL | self.lcd.display_control)
            .await?;
        Ok(())
    }

    pub async fn no_display(&mut self) -> Result<(), T::Error> {
        self.lcd.display_control &= !LCD_DISPLAYON;
        self.lcd
            .command(LCD_DISPLAYCONTROL | self.lcd.display_control)
            .await?;
        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), T::Error> {
        self.lcd.command(LCD_CLEARDISPLAY).await?; // clear display, set cursor position to zero
        self.lcd.sleep.sleep_for_micros(2000).await; // this command takes a long time!
        Ok(())
    }

    pub async fn home(&mut self) -> Result<(), T::Error> {
        self.lcd.command(LCD_RETURNHOME).await?; // set cursor position to zero
        self.lcd.sleep.sleep_for_micros(2000).await; // this command takes a long time!

        Ok(())
    }

    pub async fn set_cursor(
        &mut self,
        col: impl Into<u8>,
        row: impl Into<u8>,
    ) -> Result<(), T::Error> {
        let row_offsets: [u8; 4] = [0x00, 0x40, 0x14, 0x54];
        let mut row = row.into();
        if row > self.lcd.rows {
            row = self.lcd.rows - 1; // we count rows starting w/0
        }
        self.lcd
            .command(LCD_SETDDRAMADDR | (col.into() + row_offsets[row as usize]))
            .await?;

        Ok(())
    }

    // Turns the underline cursor on/off
    pub async fn no_cursor(&mut self) -> Result<(), T::Error> {
        self.lcd.display_control &= !LCD_CURSORON;
        self.lcd
            .command(LCD_DISPLAYCONTROL | self.lcd.display_control)
            .await?;
        Ok(())
    }
    pub async fn cursor(&mut self) -> Result<(), T::Error> {
        self.lcd.display_control |= LCD_CURSORON;
        self.lcd
            .command(LCD_DISPLAYCONTROL | self.lcd.display_control)
            .await?;
        Ok(())
    }

    // Turn on and off the blinking cursor
    pub async fn no_blink(&mut self) -> Result<(), T::Error> {
        self.lcd.display_control &= !LCD_BLINKON;
        self.lcd
            .command(LCD_DISPLAYCONTROL | self.lcd.display_control)
            .await?;

        Ok(())
    }
    pub async fn blink(&mut self) -> Result<(), T::Error> {
        self.lcd.display_control |= LCD_BLINKON;
        self.lcd
            .command(LCD_DISPLAYCONTROL | self.lcd.display_control)
            .await?;
        Ok(())
    }

    // These commands scroll the display without changing the RAM
    pub async fn scroll_display_left(&mut self) -> Result<(), T::Error> {
        self.lcd
            .command(LCD_CURSORSHIFT | LCD_DISPLAYMOVE | LCD_MOVELEFT)
            .await?;
        Ok(())
    }
    pub async fn scroll_display_right(&mut self) -> Result<(), T::Error> {
        self.lcd
            .command(LCD_CURSORSHIFT | LCD_DISPLAYMOVE | LCD_MOVERIGHT)
            .await?;
        Ok(())
    }

    // This is for text that flows Left to Right
    pub async fn left_to_right(&mut self) -> Result<(), T::Error> {
        self.lcd.display_mode |= LCD_ENTRYLEFT;
        self.lcd
            .command(LCD_ENTRYMODESET | self.lcd.display_mode)
            .await?;
        Ok(())
    }

    // This is for text that flows Right to Left
    pub async fn right_to_left(&mut self) -> Result<(), T::Error> {
        self.lcd.display_mode &= !LCD_ENTRYLEFT;
        self.lcd
            .command(LCD_ENTRYMODESET | self.lcd.display_mode)
            .await?;
        Ok(())
    }

    // This will 'right justify' text from the cursor
    pub async fn autoscroll(&mut self) -> Result<(), T::Error> {
        self.lcd.display_mode |= LCD_ENTRYSHIFTINCREMENT;
        self.lcd
            .command(LCD_ENTRYMODESET | self.lcd.display_mode)
            .await?;
        Ok(())
    }

    // This will 'left justify' text from the cursor
    pub async fn no_autoscroll(&mut self) -> Result<(), T::Error> {
        self.lcd.display_mode &= !LCD_ENTRYSHIFTINCREMENT;
        self.lcd
            .command(LCD_ENTRYMODESET | self.lcd.display_mode)
            .await?;
        Ok(())
    }

    // Turn the (optional) backlight off/on
    pub async fn no_backlight(&mut self) -> Result<(), T::Error> {
        self.lcd.backlight = LcdBacklight::NoBacklight;
        self.lcd.expander_write(0).await?;
        Ok(())
    }

    pub async fn backlight(&mut self) -> Result<(), T::Error> {
        self.lcd.backlight = LcdBacklight::Backlight;
        self.lcd.expander_write(0).await?;
        Ok(())
    }

    pub async fn print(&mut self, string: &str) -> Result<(), T::Error> {
        for c in string.as_bytes().iter() {
            self.lcd.write(*c).await?;
        }
        Ok(())
    }

    pub async fn print_bytes(&mut self, bytes: &[u8]) -> Result<(), T::Error> {
        for c in bytes {
            self.lcd.write(*c).await?;
        }

        Ok(())
    }
}
