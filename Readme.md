A utility to link leds on Viper TQS Mission Pack to MSFS2020

## Requirements for building

* Rust compiler and Visual Studio prerequisites ( https://rustup.rs/ )

* MSFS2020 SDK (enable developer mode in MSFS, download via the devoper top bar > Help > SDK installer (core) )

## How to use

1. Compile the utility (`cargo build --release`)

2. Run `leds.tmc` scrip in the Target Script Editor

3. Launch MSFS2020 and wait for it to load the main menu

4. Launch `target\release\msfs2020_leds.exe`

