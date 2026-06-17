# STM32F446RE Embassy Project

Proiect Rust embedded cu Embassy pentru placa Nucleo-F446RE.

## Structura proiectului

```
stm32f446re-project/
├── .cargo/
│   └── config.toml       # Target + runner (probe-rs)
├── src/
│   └── main.rs           # Codul principal
├── Cargo.toml            # Dependențe
├── build.rs              # Linker script setup
├── memory.x              # Flash/RAM layout pentru STM32F446RE
├── rust-toolchain.toml   # Nightly Rust
└── .gitignore
```

## Setup (prima dată)

### 1. Instalează Rust nightly + target
```bash
rustup toolchain install nightly
rustup target add thumbv7em-none-eabihf
```

### 2. Instalează uneltele necesare
```bash
cargo install flip-link
cargo install probe-rs-tools --locked
```

### 3. Verifică că placa e detectată
```bash
probe-rs list
```

## Build & Flash

```bash
# Build debug
cargo build

# Build release
cargo build --release

# Flash + run (cu logs defmt)
cargo run

# Sau explicit
cargo run --release
```

## Pinout Nucleo-F446RE

| Pin  | Funcție            |
|------|--------------------|
| PA5  | LED verde (LD2)    |
| PC13 | Buton USER (B1)    |
| PA2  | USART2 TX          |
| PA3  | USART2 RX          |

## Memorie STM32F446RE

- **Flash**: 512 KB @ 0x08000000
- **SRAM**: 128 KB @ 0x20000000
- **Core**: Cortex-M4F (cu FPU)
- **Target Rust**: `thumbv7em-none-eabihf`
