# My attempt at Rust on an Arduino Uno R4 Wifi board

I used the app-template from knurling-rs

```bash
cargo generate \
    --git https://github.com/knurling-rs/app-template \
    --branch main \
    --name my-app
```

### 2. Set `probe-rs` chip

If, for example, you have an Arduino Uno R4 Wifi, replace `{{chip}}` with `R7FA4M1AB`.

```diff
 # .cargo/config.toml
-runner = ["probe-rs", "run", "--chip", "$CHIP", "--log-format=oneline"]
+runner = ["probe-rs", "run", "--chip", "R7FA4M1AB", "--log-format=oneline"]
```

### 3. Adjust the compilation target

In `.cargo/config.toml`, pick the right compilation target for your board.

```diff
 # .cargo/config.toml
 [build]
-target = "thumbv6m-none-eabi"    # Cortex-M0 and Cortex-M0+
-# target = "thumbv7m-none-eabi"    # Cortex-M3
-# target = "thumbv7em-none-eabi"   # Cortex-M4 and Cortex-M7 (no FPU)
-# target = "thumbv7em-none-eabihf" # Cortex-M4F and Cortex-M7F (with FPU)
+target = "thumbv7em-none-eabihf" # Cortex-M4F (with FPU)
```

Add the target with `rustup`.

```bash
rustup target add thumbv7em-none-eabihf
```

### 4. Add a HAL as a dependency

In `Cargo.toml`, list the ~~Hardware Abstraction Layer (HAL)~~ PAC in this case for your board as a dependency.

For the R7FA4M1AB you'll want to use the [`ra4m1`] pac.

[`ra4m1`]: https://crates.io/crates/ra4m1

```diff
 # Cargo.toml
 [dependencies]
-# some-hal = "1.2.3"
+ra4m1 = { version = "0.2.1", git="https://github.com/ra-rs/ra", features = [ "rt"] }
```
### 5. Import your HAL

Now that you have selected a HAL, fix the HAL import in `src/lib.rs`

```diff
 // my-app/src/lib.rs
-// use some_hal as _; // memory layout
+use ra4m1 as _; // memory layout
```

### (6. Get a linker script)

Since this is an Arduino board with an Arduino bootloader, we use a slightly modified `memory.x` file. If we start the flash at 0x00004000 we can avoid overwriting the Arduino bootloader and having to constantly fix the USB connectivity (ask me how I know...)

There is also the possibility that I need to include the "options" section as well in order to initialize the board properly, but I'm not sure how that works.

The `memory.x` file should look something like:

```text
MEMORY
{
  FLASH : ORIGIN = 0x00004000, LENGTH = 240K
  RAM   : ORIGIN = 0x20000000, LENGTH = 32K
  DFLASH: ORIGIN = 0x40100000, LENGTH = 8K
}
```

The `memory.x` file is included in the `cortex-m-rt` linker script `link.x`, and so `link.x` is the one you should tell `rustc` to use (see the `.cargo/config.toml` file where we do that).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

[Knurling]: https://knurling.ferrous-systems.com
[Ferrous Systems]: https://ferrous-systems.com/
[GitHub Sponsors]: https://github.com/sponsors/knurling-rs
