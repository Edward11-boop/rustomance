# Rustomance 💝

An async, `no_std` embedded Rust project running on an STM32 microcontroller, built as a personal birthday gift.

It drives an ILI9341 TFT display over SPI and renders a small interactive multi-page UI, fully navigable with physical buttons — no heap, no `std`, just async tasks talking to each other through channels.

## What it does

- Boots into a home screen showing a list of messages
- Navigate the list with **Up** / **Down** buttons
- Press **Confirm** to open a dedicated page for the selected message
- Press **Cancel** to go back to the home screen
- Each page can be customized independently (text, colors, future animations/images)

## Why "Rustomance"

Rust + Romance. It's a love letter written in `no_std` Rust, flashed onto a microcontroller instead of paper.

## Architecture

The whole thing is built around [Embassy](https://embassy.dev/), an async embedded framework for Rust. Instead of one big blocking loop, the logic is split into independent async tasks that communicate through `embassy_sync::channel::Channel`:

```
 [ Buttons ]                  [ State / Logic ]                [ Display ]
 homeSender   --- But --->     homeReceiver    --- Display --->   main loop
 (task)         CHANNEL        (task)            DISPLAY_CHANNEL  (rendering)
```

- **`homeSender`** polls the GPIO buttons (with simple debouncing) and sends raw button events (`Up`, `Down`, `Confirm`, `Cancel`) on `CHANNEL`.
- **`homeReceiver`** owns the application state (selected index, current screen) and decides what to do with each button event, then sends a `Display` enum (`Home(usize)` or `Page(usize)`) on `DISPLAY_CHANNEL`.
- **`main`** owns the actual display driver and only reacts to `DISPLAY_CHANNEL`, rendering whatever screen it's told to render.

This separation keeps button polling, state/logic, and rendering decoupled — each piece only knows what it needs to know.

## Hardware

- STM32 microcontroller (Embassy HAL)
- ILI9341 TFT display, 240x320, connected over SPI (rotated to landscape)
- 4 push buttons: Up, Down, Confirm, Cancel

| Signal       | Pin   |
|--------------|-------|
| SPI SCK      | PB13  |
| SPI MOSI     | PB15  |
| SPI MISO     | PB14  |
| Display CS   | PB12  |
| Display DC   | PB1   |
| Display RST  | PB2   |
| Button Up    | PA9   |
| Button Down  | PA8   |
| Button Confirm | PB10 |
| Button Cancel  | PB4  |

All buttons use the internal pull-up resistor (`Pull::Up`) — wire each button between its pin and GND.

## Tech stack

- [`embassy-stm32`](https://github.com/embassy-rs/embassy) — async HAL and executor
- [`mipidsi`](https://crates.io/crates/mipidsi) — ILI9341 display driver
- [`embedded-graphics`](https://crates.io/crates/embedded-graphics) — drawing primitives and text
- [`embedded-hal-bus`](https://crates.io/crates/embedded-hal-bus) — shared SPI device handling
- [`heapless`](https://crates.io/crates/heapless) — `no_std` dynamic-ish strings, no heap allocation
- [`defmt`](https://crates.io/crates/defmt) + `defmt-rtt` — logging over RTT
- `panic-probe` — panic handling for embedded targets

## Status

Work in progress currently each page only shows its title. Next steps include adding per-page content: animations, a countdown/timeline, and a photo slideshow read from an SD card.

Built with ❤️ (and a fair amount of borrow-checker fighting).