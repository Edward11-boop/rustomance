#![no_std]
#![no_main]
#![allow(non_snake_case)]

use embassy_stm32::peripherals::{PA8, PA9, PB10, PB4};
use core::fmt::Write;
use heapless::String;
use embassy_stm32::gpio::{Output, Input, Level, Speed, Pull};
use embassy_stm32::time::Hertz;
use embassy_stm32::spi::{Spi, Config as SpiConfig};
use embassy_stm32::dma::NoDma;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use mipidsi::options::{Orientation, Rotation};
use embassy_time::{Delay, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::Builder;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ILI9341Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::text::Text;
use embedded_graphics::primitives::{Rectangle, PrimitiveStyle};
use defmt_rtt as _;
use panic_probe as _;
use embassy_executor::Spawner;
use defmt::Format;
use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

static CHANNEL: Channel<ThreadModeRawMutex, Button, 5> = Channel::new();
static DISPLAY_CHANNEL: Channel<ThreadModeRawMutex, Screen, 1> = Channel::new();

#[derive(Format)]
enum Button {
    Up,
    Down,
    Confirm,
    Cancel,
}

#[derive(Format, Clone, Copy)]
enum Screen {
    Home(usize),
    Page(usize, usize),
}

const MENU_ITEMS: [&str; 4] = [
    "1. La multi ani iubita mea !",
    "2. Vreau sa stii ca te iubesc !",
    "3. Vom face 3 ani impreuna !",
    "4. Ce repede trece timpul ...",
];

const LOVE_MESSAGE: [&str; 3] = [
    "In fiecare dimineata cu tine\nZambesc fara motiv\nEsti cafeaua mea cu iubire\nSi haosul meu preferat definitiv.",
    "Nu-mi trebuie cuvinte mari\nCand tu ma faci sa rad\nEsti linistea mea dulce\nSi motivul meu cel mai bland.",
    "La multi ani iubirea mea\nTe ador nespus de tare\nMultumesc ca ma suporti zilnic\nCu zambet pupici si rabdare.",
];


const HEART: [[u8; 7]; 6] = [
    [0, 1, 1, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 0],
    [0, 0, 1, 1, 1, 0, 0],
    [0, 0, 0, 1, 0, 0, 0],
];

#[embassy_executor::task]
async fn button_sender(
    next: Input<'static, PA9>,
    previous: Input<'static, PA8>,
    confirm: Input<'static, PB10>,
    cancel: Input<'static, PB4>,
) {
    loop {
        if next.is_low() {
            while next.is_low() { Timer::after_millis(50).await; }
            CHANNEL.send(Button::Up).await;
        } else if previous.is_low() {
            while previous.is_low() { Timer::after_millis(50).await; }
            CHANNEL.send(Button::Down).await;
        } else if confirm.is_low() {
            while confirm.is_low() { Timer::after_millis(50).await; }
            CHANNEL.send(Button::Confirm).await;
        } else if cancel.is_low() {
            while cancel.is_low() { Timer::after_millis(50).await; }
            CHANNEL.send(Button::Cancel).await;
        }
        Timer::after_millis(10).await;
    }
}

#[embassy_executor::task]
async fn screen_controller() {
    let mut counter: usize = 0;
    let mut sub_page: usize = 0;

    let mut on_home: bool = true;

    loop {
        while let Ok(button) = CHANNEL.try_receive() {
            match button {
                Button::Up => {
                    if on_home {
                        if counter >= MENU_ITEMS.len() - 1 { counter = 0; }
                        else { counter += 1; }
                        DISPLAY_CHANNEL.send(Screen::Home(counter)).await;
                    } else if counter == 0 {
                        if sub_page >= LOVE_MESSAGE.len() - 1 { sub_page = 0; }
                        else { sub_page += 1; }
                        DISPLAY_CHANNEL.send(Screen::Page(counter, sub_page)).await;
                    }
                }
                Button::Down => {
                    if on_home {
                        if counter == 0 { counter = MENU_ITEMS.len() - 1; }
                        else { counter -= 1; }
                        DISPLAY_CHANNEL.send(Screen::Home(counter)).await;
                    } else if counter == 0 {
                        if sub_page == 0 { sub_page = LOVE_MESSAGE.len() - 1; }
                        else { sub_page -= 1; }
                        DISPLAY_CHANNEL.send(Screen::Page(counter, sub_page)).await;
                    }
                }
                Button::Confirm => {
                    if on_home {
                        on_home = false;
                        sub_page = 0;
                        DISPLAY_CHANNEL.send(Screen::Page(counter, sub_page)).await;
                    }
                }
                Button::Cancel => {
                    on_home = true;
                    DISPLAY_CHANNEL.send(Screen::Home(counter)).await;
                }
            }
        }
        Timer::after_millis(10).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let next = Input::new(p.PA9, Pull::Up);
    let previous = Input::new(p.PA8, Pull::Up);
    let confirm = Input::new(p.PB10, Pull::Up);
    let cancel = Input::new(p.PB4, Pull::Up);

    let mut config = SpiConfig::default();
    config.frequency = Hertz(8_000_000);

    let spi = Spi::new(p.SPI2, p.PB13, p.PB15, p.PB14, NoDma, NoDma, config);

    let cs = Output::new(p.PB12, Level::High, Speed::High);
    let dc = Output::new(p.PB1, Level::High, Speed::High);
    let rst = Output::new(p.PB2, Level::High, Speed::High);

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut buffer = [0u8; 512];
    let interface = SpiInterface::new(spi_device, dc, &mut buffer);
    let mut delay = Delay;

    let mut display = Builder::new(ILI9341Rgb565, interface)
        .display_size(240, 320)
        .orientation(Orientation::new().rotate(Rotation::Deg270))
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();

    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let style_title = MonoTextStyle::new(&FONT_10X20, Rgb565::YELLOW);
    let style_pink = MonoTextStyle::new(&FONT_10X20, Rgb565::MAGENTA);

    spawner.spawn(button_sender(next, previous, confirm, cancel)).unwrap();
    spawner.spawn(screen_controller()).unwrap();

    display.clear(Rgb565::BLACK).unwrap();
    DISPLAY_CHANNEL.send(Screen::Home(0)).await;

    loop {
        let selected = DISPLAY_CHANNEL.receive().await;

        display.clear(Rgb565::BLACK).unwrap();

        match selected {
            Screen::Home(counter) => {
                for (i, item) in MENU_ITEMS.iter().enumerate() {
                    let mut text: String<64> = String::new();
                    if i == counter {
                        write!(text, "-> {}", item).ok();
                    } else {
                        write!(text, "   {}", item).ok();
                    }
                    Text::new(
                        &text,
                        Point::new(5, (i as i32 * 40) + 60),
                        style,
                    ).draw(&mut display).unwrap();
                }
            }
            Screen::Page(counter, sub_page) => {
                if counter == 0 {
                    Rectangle::new(Point::new(5, 5), Size::new(310, 230))
                        .into_styled(PrimitiveStyle::with_stroke(Rgb565::MAGENTA, 2))
                        .draw(&mut display).unwrap();

                    Rectangle::new(Point::new(10, 10), Size::new(300, 220))
                        .into_styled(PrimitiveStyle::with_stroke(Rgb565::YELLOW, 1))
                        .draw(&mut display).unwrap();

                    let pixel_size: i32 = 6;
                    let start_x: i32 = 130;
                    let start_y: i32 = 15;

                    for (row, line) in HEART.iter().enumerate() {
                        for (col, value) in line.iter().enumerate() {
                            if *value == 1 {
                                Rectangle::new(
                                    Point::new(
                                        start_x + (col as i32 * pixel_size),
                                        start_y + (row as i32 * pixel_size),
                                    ),
                                    Size::new(pixel_size as u32, pixel_size as u32),
                                ).into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
                                 .draw(&mut display).unwrap();
                            }
                        }
                    }
                    for offset in 0..2 {
                        Text::new(
                            LOVE_MESSAGE[sub_page],
                            Point::new(15 + offset, 80),
                            style_pink,
                        ).draw(&mut display).unwrap();
                    }
                } else {
                    Text::new(
                        MENU_ITEMS[counter],
                        Point::new(20, 30),
                        style_title,
                    ).draw(&mut display).unwrap();
                }

                Text::new(
                    "<- Back ",
                    Point::new(5, 220),
                    style,
                ).draw(&mut display).unwrap();
            }
        }
    }
}