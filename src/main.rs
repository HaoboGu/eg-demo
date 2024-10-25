#![no_main]
#![no_std]

mod rm67162;
mod w25q;

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    mode::Blocking,
    ospi::{self, ChipSelectHighTime, MemorySize},
    time::Hertz,
    Config,
};
use embassy_stm32::{mode::Async, ospi::Ospi, peripherals::OCTOSPI1};
use embedded_graphics::{
    draw_target::{DrawTarget, DrawTargetExt as _},
    framebuffer::Framebuffer,
    geometry::{Point, Size},
    image::ImageDrawable,
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::{raw::BigEndian, Rgb565, RgbColor, WebColors},
    primitives::Rectangle,
    text::{Alignment, Text},
};
// use embassy_sync::blocking_mutex::{raw::NoopRawMutex, NoopMutex};
// use embassy_time::Delay;
// use embassy_time::Timer;
// use embedded_graphics::{
//     draw_target::DrawTarget,
//     geometry::Point,
//     image::{Image, ImageRawLE},
//     mono_font::{ascii::FONT_10X20, MonoTextStyle},
//     pixelcolor::Rgb888,
//     pixelcolor::{Rgb565, RgbColor},
//     prelude::*,
//     primitives::Rectangle,
//     primitives::{Line, Primitive as _, PrimitiveStyle},
//     text::Text,
//     Drawable,
// };
use panic_probe as _;
use rm67162::RM67162;

// static SPI_BUS: StaticCell<NoopMutex<RefCell<Spi<SPI1, DMA1_CH1, NoDma>>>> = StaticCell::new();
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // RCC config
    let mut config = Config::default();
    info!("START");
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi = Some(HSIPrescaler::DIV1);
        config.rcc.csi = true;
        // Needed for USB
        config.rcc.hsi48 = Some(Hsi48Config {
            sync_from_usb: true,
        });
        // External oscillator 25MHZ
        config.rcc.hse = Some(Hse {
            freq: Hertz(25_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV5,
            mul: PllMul::MUL112,
            divp: Some(PllDiv::DIV2),
            divq: Some(PllDiv::DIV2),
            divr: Some(PllDiv::DIV2),
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV2;
        config.rcc.apb1_pre = APBPrescaler::DIV2;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
        config.rcc.apb3_pre = APBPrescaler::DIV2;
        config.rcc.apb4_pre = APBPrescaler::DIV2;
        config.rcc.voltage_scale = VoltageScale::Scale0;
    }

    // Initialize peripherals
    let p = embassy_stm32::init(config);

    let qspi_config = embassy_stm32::ospi::Config {
        fifo_threshold: FIFOThresholdLevel::_16Bytes,
        memory_type: MemoryType::Micron,
        device_size: MemorySize::_8MiB,
        chip_select_high_time: ChipSelectHighTime::_1Cycle,
        free_running_clock: false,
        clock_mode: false,
        wrap_size: WrapSize::None,
        clock_prescaler: 2,
        sample_shifting: true,
        delay_hold_quarter_cycle: false,
        chip_select_boundary: 0,
        delay_block_bypass: true,
        max_transfer: 0,
        refresh: 0,
    };
    let ospi = embassy_stm32::ospi::Ospi::new_blocking_quadspi(
        p.OCTOSPI1,
        p.PB2,
        p.PD11,
        p.PD12,
        p.PE2,
        p.PD13,
        p.PB6,
        qspi_config,
    );

    let mut flash = FlashMemory::new(ospi).await;

    let flash_id = flash.read_id();
    info!("FLASH ID: {=[u8]:x}", flash_id);
    // let mut wr_buf = [0u8; 256];
    // for i in 0..256 {
    //     wr_buf[i] = (i + 2) as u8;
    // }
    let mut rd_buf = [0u8; 256];
    // flash.erase_sector(0).await;
    // flash.write_memory(0, &wr_buf, true).await;
    flash.read_memory(0, &mut rd_buf, true);
    // info!("WRITE BUF: {:?}", wr_buf);
    info!("READ BUF: {=[u8]:#X}", rd_buf);
    info!("End of Program, proceed to empty endless loop");
    flash.enable_mm().await;

    // Output pin PE3
    let mut led = Output::new(p.PE3, Level::Low, Speed::Low);

    loop {
        // Print the u32 at 0x90000000
        defmt::info!("0x90000000: {=u32:#X}", unsafe {
            *(0x90000000 as *const u32)
        });
        defmt::info!("0x90000004: {=u32:#X}", unsafe {
            *(0x90000004 as *const u32)
        });
        defmt::info!("0x90000008: {=u32:#X}", unsafe {
            *(0x90000008 as *const u32)
        });
        defmt::info!("0x9000000c: {=u32:#X}", unsafe {
            *(0x9000000c as *const u32)
        });
        defmt::info!("0x90000010: {=u32:#X}", unsafe {
            *(0x90000010 as *const u32)
        });
        led.toggle();
        Timer::after_millis(1000).await;
    }

    // rm67162
    // let mut vbat = Output::new(p.PB11, Level::High, Speed::Low);
    // let mut vcc: Output = Output::new(p.PB10, Level::High, Speed::Low);
    // let mut im = Output::new(p.PE15, Level::High, Speed::VeryHigh);
    // vbat.set_high();
    // vcc.set_high();
    // im.set_high();

    // let ospi_config = ospi::Config {
    //     clock_prescaler: 1,
    //     device_size: MemorySize::_128MiB,
    //     chip_select_high_time: ChipSelectHighTime::_1Cycle,
    //     ..Default::default()
    // };

    // let ospi: Ospi<OCTOSPI1, Blocking> = Ospi::new_blocking_quadspi(
    //     p.OCTOSPI1,
    //     p.PB2,
    //     p.PC9,
    //     p.PD12,
    //     p.PE2,
    //     p.PD13,
    //     p.PB6,
    //     ospi_config,
    // );

    // let mut rm67162 = RM67162::new(ospi);
    // let mut rst = Output::new(p.PD3, Level::High, Speed::Low);
    // rm67162.reset(&mut rst, &mut embassy_time::Delay).unwrap();
    // info!("reset display");
    // rm67162.init(&mut embassy_time::Delay).unwrap();

    // rm67162.clear(Rgb565::WHITE).unwrap();

    // let gif = tinygif::Gif::from_slice(include_bytes!("../ferris3.gif")).unwrap();

    // let mut fb = Framebuffer::<
    //     Rgb565,
    //     _,
    //     BigEndian,
    //     536,
    //     240,
    //     { embedded_graphics::framebuffer::buffer_size::<Rgb565>(536, 240) },
    // >::new();
    // fb.clear(Rgb565::WHITE).unwrap();

    // loop {
    //     for frame in gif.frames() {
    //         frame.draw(&mut fb.translated(Point::new(0, 0))).unwrap();
    //         // println!("draw frame {:?}", frame);
    //         unsafe {
    //             rm67162.fill_with_framebuffer(fb.data()).unwrap();
    //         }
    //         // info!("tick");
    //     }
    // }
}
