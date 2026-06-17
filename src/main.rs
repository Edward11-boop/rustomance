#![no_std]
#![no_main]
#![allow(non_snake_case)]

use embassy_stm32::peripherals::{PA8, PA9,PB10,PB4};
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
use defmt_rtt as _;
use panic_probe as _;
use embassy_executor::Spawner;
use defmt::Format;
use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

static CHANNEL:         Channel<ThreadModeRawMutex, But,   5> = Channel::new();
static DISPLAY_CHANNEL: Channel<ThreadModeRawMutex, Display, 1> = Channel::new();

#[derive(Format)]
enum But {
    Up,
    Down,
    Confirm,
    Cancel,
}

#[derive(Format,Clone,Copy)]
enum Display {
    Home(usize),
    Page(usize),
}


#[embassy_executor::task]
async fn homeSender(
    next:     Input<'static, PA9>,
    previous: Input<'static, PA8>,
    confirm : Input<'static,PB10>,
    cancel : Input <'static,PB4>,
) {
    loop {
        if next.is_low() {
            while next.is_low() { Timer::after_millis(50).await; }
            CHANNEL.send(But::Up).await;
        } 
        else if previous.is_low() {
            while previous.is_low() { Timer::after_millis(50).await; }
            CHANNEL.send(But::Down).await;
        }
        else if confirm.is_low(){
            while confirm.is_low(){
                Timer::after_millis(50).await;
            }
            CHANNEL.send(But::Confirm).await;
        }
        else if cancel.is_low(){
            while cancel.is_low(){
                Timer::after_millis(50).await;
            }
            CHANNEL.send(But::Cancel).await;
        }
        Timer::after_millis(10).await;
    }
}

#[embassy_executor::task]
async fn homeReceiver() {
    let mut counter: usize = 0;

    let messages = [
        "1. La multi ani iubita mea !",
        "2. Vreau sa stii ca te iubesc !",
        "3. Vom face 3 ani impreuna !",
        "4. Ce repede trece timpul ...",
    ];
    
    let mut on_home : bool = true;

    loop {
        while let Ok(buton) = CHANNEL.try_receive() {
            match buton {
                But::Up => {
                    if on_home {
                        if counter >= messages.len() - 1 { counter = 0; }
                        else { counter += 1; }
                        DISPLAY_CHANNEL.send(Display::Home(counter)).await;
                    }
                }
                But::Down => {
                    if on_home {
                        if counter == 0 { counter = messages.len() - 1; }
                        else { counter -= 1; }
                        DISPLAY_CHANNEL.send(Display::Home(counter)).await;
                    }
                }
                But::Confirm => {
                    if on_home {
                        on_home = false;
                        DISPLAY_CHANNEL.send(Display::Page(counter)).await;
                    }
                }
                But::Cancel => {
                        on_home = true;
                        DISPLAY_CHANNEL.send(Display::Home(counter)).await;
                }
            }
        }
        Timer::after_millis(10).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let next     = Input::new(p.PA9, Pull::Up);
    let previous = Input::new(p.PA8, Pull::Up);
    let confirm = Input::new(p.PB10, Pull::Up);
    let cancel = Input::new(p.PB4, Pull::Up);


    let mut config = SpiConfig::default();
    config.frequency = Hertz(8_000_000);

    let spi = Spi::new(
        p.SPI2,
        p.PB13,
        p.PB15,
        p.PB14,
        NoDma,
        NoDma,
        config,
    );

    let cs  = Output::new(p.PB12, Level::High, Speed::High);
    let dc  = Output::new(p.PB1,  Level::High, Speed::High);
    let rst = Output::new(p.PB2,  Level::High, Speed::High);

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut buffer = [0u8; 512];
    let interface  = SpiInterface::new(spi_device, dc, &mut buffer);
    let mut delay  = Delay;

    let mut display = Builder::new(ILI9341Rgb565, interface)
        .display_size(240, 320)
        .orientation(Orientation::new().rotate(Rotation::Deg270))
        .reset_pin(rst)
        .init(&mut delay)
        .unwrap();

    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let style_title = MonoTextStyle::new(&FONT_10X20, Rgb565::YELLOW);

    let messages = [
        "1. La multi ani iubita mea !",
        "2. Vreau sa stii ca te iubesc !",
        "3. Vom face 3 ani impreuna !",
        "4. Ce repede trece timpul ...",
    ];

    spawner.spawn(homeSender(next, previous,confirm,cancel)).unwrap();
    spawner.spawn(homeReceiver()).unwrap();

    display.clear(Rgb565::BLACK).unwrap();
    for (i, message) in messages.iter().enumerate() {
        let mut text: String<64> = String::new();
        if i == 0 {
            write!(text, "-> {}", message).ok();
        } else {
            write!(text, "   {}", message).ok();
        }
        Text::new(&text, Point::new(5, (i as i32 * 40) + 60), style)
            .draw(&mut display)
            .unwrap();
    }


    loop {
        let selected = DISPLAY_CHANNEL.receive().await;

        display.clear(Rgb565::BLACK).unwrap(); // clear the display 

        match selected {
            Display::Home(counter) => {
                for (i , line) in messages.iter().enumerate(){
                    let mut text : String<64> = String::new();
                    if i == counter {
                        write!(text,"-> {}",line).ok();
                    }
                    else {
                        write!(text," {}",line).ok();
                    }
                    Text::new(
                        &text,
                        Point::new(5, ( i as i32 * 40) + 60),
                        style
                    ).draw(&mut display).unwrap();
                }
            }
            Display::Page(counter) => {
                match counter {
                    0 => {
                        Text::new(
                            "1. La multi ani iubita mea !",
                            Point::new(20, 30),
                            style_title,
                        ).draw(&mut display).unwrap();
                    }
                    1 => {
                        Text::new(
                            "2. La multi ani iubita mea !",
                            Point::new(20, 30),
                            style_title,
                        ).draw(&mut display).unwrap();
                    }
                    2 => {
                        Text::new(
                            "3. La multi ani iubita mea !",
                            Point::new(20, 30),
                            style_title,
                        ).draw(&mut display).unwrap();
                    }
                    _ => {
                        Text::new(
                            "4. La multi ani iubita mea !",
                            Point::new(20, 30),
                            style_title,
                        ).draw(&mut display).unwrap();
                    }
                }
            }
        }
    }
}