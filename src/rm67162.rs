//! On-board RM67162 AMOLED screen driver

use embedded_graphics::{
    pixelcolor::{raw::ToBytes, Rgb565},
    prelude::{DrawTarget, OriginDimensions, Size},
    primitives::Rectangle,
    Pixel,
};
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use embassy_stm32::{mode::Async, ospi::Ospi, peripherals::OCTOSPI1};
use embassy_stm32::ospi::*;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Orientation {
    Portrait,
    Landscape,
    PortraitFlipped,
    LandscapeFlipped,
}

impl Orientation {
    pub(crate) fn to_madctr(&self) -> u8 {
        match self {
            Orientation::Portrait => 0x00,
            Orientation::PortraitFlipped => 0b11000000,
            Orientation::Landscape => 0b01100000,
            Orientation::LandscapeFlipped => 0b10100000,
        }
    }
}

pub struct RM67162<'a> {
    ospi: Ospi<'a, OCTOSPI1, Async>,
    orientation: Orientation,
}

impl RM67162<'_>
{
    pub fn new<'a>(ospi: Ospi<'a, OCTOSPI1, Async>) -> RM67162<'a> {
        RM67162 {
            ospi,
            orientation: Orientation::Portrait,
        }
    }

    pub fn set_orientation(&mut self, orientation: Orientation) -> Result<(), OspiError> {
        self.orientation = orientation;
        self.send_cmd(0x36, &[self.orientation.to_madctr()])
    }

    pub fn reset(&self, rst: &mut impl OutputPin, delay: &mut impl DelayNs) -> Result<(), OspiError> {
        rst.set_low().unwrap();
        delay.delay_ms(300);

        rst.set_high().unwrap();
        delay.delay_ms(200);
        Ok(())
    }

    /// send 1-1-1 command by default
    fn send_cmd(&mut self, cmd: u32, data: &[u8]) -> Result<(), OspiError> {
        // TODO: Do we need cs?
        // self.cs.set_low().unwrap();
        let mut transfer_config = TransferConfig::default();
        transfer_config.instruction = Some(0x02);
        transfer_config.address = Some(0 | (cmd << 8));
        transfer_config.adsize = AddressSize::_24bit;
        // NbData: calculated automatically
        transfer_config.iwidth = OspiWidth::SING;
        transfer_config.adwidth = OspiWidth::SING;
        transfer_config.dwidth = OspiWidth::SING;
        transfer_config.dummy = DummyCycles::_0;

        transfer_config.abwidth = OspiWidth::NONE;
        transfer_config.ddtr = false;

        if data.len() == 0 {
            // TODO: Check this cfg when data is empty
            transfer_config.address = Some(0 | cmd);
            transfer_config.adsize = AddressSize::_16Bit;
            self.ospi.blocking_write_dma(&[0_u8], transfer_config)?;
        } else {
            self.ospi.blocking_write_dma(data, transfer_config)?;
        }
        // self.cs.set_high().unwrap();
        Ok(())
    }

    fn send_cmd_114(&mut self, cmd: u32, data: &[u8]) -> Result<(), OspiError> {
        // self.cs.set_low().unwrap();
        let mut transfer_config = TransferConfig::default();
        transfer_config.instruction = Some(0x32);
        transfer_config.address = Some(0 | (cmd << 8));
        transfer_config.adsize = AddressSize::_24bit;
        // NbData: calculated automatically
        transfer_config.iwidth = OspiWidth::SING;
        transfer_config.adwidth = OspiWidth::SING;
        transfer_config.dwidth = OspiWidth::QUAD;
        transfer_config.dummy = DummyCycles::_0;

        transfer_config.abwidth = OspiWidth::NONE;
        transfer_config.ddtr = false;
        if data.len() == 0 {
            // TODO: Check this cfg when data is empty
            transfer_config.address = Some(0 | cmd);
            transfer_config.adsize = AddressSize::_16Bit;
            transfer_config.dwidth = OspiWidth::SING;
            self.ospi.blocking_write_dma(&[0_u8], transfer_config)?;
        } else {
            self.ospi.blocking_write_dma(data, transfer_config)?;
        }

        // self.cs.set_high().unwrap();
        Ok(())
    }

    fn send_data_114(&mut self, data: &[u8]) -> Result<(), OspiError> {
        // self.cs.set_low().unwrap();
        let mut transfer_config = TransferConfig::default();
        transfer_config.instruction = None;
        transfer_config.address = None;
        // NbData: calculated automatically
        transfer_config.iwidth = OspiWidth::SING;
        transfer_config.adwidth = OspiWidth::SING;
        transfer_config.dwidth = OspiWidth::QUAD;
        transfer_config.dummy = DummyCycles::_0;

        transfer_config.abwidth = OspiWidth::NONE;
        transfer_config.ddtr = false;

        self.ospi.blocking_write_dma(data, transfer_config)?;

        // self.cs.set_high().unwrap();
        Ok(())
    }


    /// rm67162_ospi_init
    pub fn init(&mut self, delay: &mut impl embedded_hal::delay::DelayNs) -> Result<(), OspiError> {
        // TODO: Reset pin, do screen reset(twice)


        // Init 3 times?
        for _ in 0..3 {
            self.send_cmd(0xEF, &[0x05])?; // sleep out
            self.send_cmd(0x05, &[0x15])?; // sleep out
            
            self.send_cmd(0x11, &[])?; // sleep out
            delay.delay_ms(120);

            self.send_cmd(0x3A, &[0x55])?; // 16bit mode

            self.send_cmd(0x51, &[0x00])?; // write brightness
            delay.delay_ms(1);

            // According to sample code, we need to set the following registers
            self.send_cmd(0x2A, &[0x00, 0x00, 0x00, 0xEF])?;
            self.send_cmd(0x2B, &[0x00, 0x00, 0x02, 0x17])?;

            self.send_cmd(0x29, &[])?; // display on
            delay.delay_ms(120);

            self.send_cmd(0x51, &[0xD0])?; // write brightness
        }

        self.set_orientation(self.orientation)?;
        Ok(())
    }

    pub fn set_address(&mut self, x1: u16, y1: u16, x2: u16, y2: u16) -> Result<(), OspiError> {
        self.send_cmd(
            0x2a,
            &[
                (x1 >> 8) as u8,
                (x1 & 0xFF) as u8,
                (x2 >> 8) as u8,
                (x2 & 0xFF) as u8,
            ],
        )?;
        self.send_cmd(
            0x2b,
            &[
                (y1 >> 8) as u8,
                (y1 & 0xFF) as u8,
                (y2 >> 8) as u8,
                (y2 & 0xFF) as u8,
            ],
        )?;
        self.send_cmd(0x2c, &[])?;
        Ok(())
    }

    pub fn draw_point(&mut self, x: u16, y: u16, color: Rgb565) -> Result<(), OspiError> {
        // self.cs.set_low().unwrap();
        self.set_address(x, y, x, y)?;
        self.send_cmd_114(0x2C, &color.to_le_bytes()[..])?;
        // self.cs.set_high().unwrap();
        Ok(())
    }

    pub fn fill_colors(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        mut colors: impl Iterator<Item = Rgb565>,
    ) -> Result<(), OspiError> {
        self.set_address(x, y, x + w - 1, y + h - 1)?;
        // self.cs.set_low().unwrap();
        self.send_cmd_114(0x2C, &colors.next().unwrap().to_be_bytes()[..])?;
        // self.spi
        //     .write(
        //         SpiDataMode::Quad,
        //         Command::Command8(0x32, SpiDataMode::Single),
        //         Address::Address24(0x2C << 8, SpiDataMode::Single),
        //         0,
        //         &colors.next().unwrap().to_be_bytes()[..],
        //     )
        //     .unwrap();

        for _ in 1..((w as u32) * (h as u32)) {

            self.send_data_114(&colors.next().unwrap().to_be_bytes()[..])?;

            // self.spi
            //     .write(
            //         SpiDataMode::Quad,
            //         Command::None,
            //         Address::None,
            //         0,
            //         &colors.next().unwrap().to_be_bytes()[..],
            //     )
            //     .unwrap();
        }
        // self.cs.set_high().unwrap();
        Ok(())
    }

    fn fill_color(&mut self, x: u16, y: u16, w: u16, h: u16, color: Rgb565) -> Result<(), OspiError> {
        self.set_address(x, y, x + w - 1, y + h - 1)?;
        // self.cs.set_low().unwrap();
        self.send_cmd_114(0x2C, &color.to_be_bytes()[..])?;
        // self.spi
        //     .write(
        //         SpiDataMode::Quad,
        //         Command::Command8(0x32, SpiDataMode::Single),
        //         Address::Address24(0x2C << 8, SpiDataMode::Single),
        //         0,
        //         &color.to_be_bytes()[..],
        //     )
        //     .unwrap();

        for _ in 1..((w as u32) * (h as u32)) {
            self.send_data_114(&color.to_be_bytes()[..])?;
            // self.spi
            //     .write(
            //         SpiDataMode::Quad,
            //         Command::None,
            //         Address::None,
            //         0,
            //         &color.to_be_bytes()[..],
            //     )
            //     .unwrap();
        }
        // self.cs.set_high().unwrap();
        Ok(())
    }
}

impl OriginDimensions for RM67162<'_>
{
    fn size(&self) -> Size {
        if matches!(
            self.orientation,
            Orientation::Landscape | Orientation::LandscapeFlipped
        ) {
            Size::new(536, 240)
        } else {
            Size::new(240, 536)
        }
    }
}

impl DrawTarget for RM67162<'_>
{
    type Color = Rgb565;

    type Error = OspiError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for Pixel(pt, color) in pixels {
            if pt.x < 0 || pt.y < 0 {
                continue;
            }
            self.draw_point(pt.x as u16, pt.y as u16, color)?;
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.fill_color(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            area.size.height as u16,
            color,
        )?;
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.fill_colors(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            area.size.height as u16,
            colors.into_iter(),
        )?;
        Ok(())
    }
}