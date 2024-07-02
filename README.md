## Dependencies

Cross compilation building is done easiest via cross library.

- [Cross](https://github.com/cross-rs/cross)

Alternatively you can setup the NDK and build manually

## Build

Make sure you have added the nessecary targets to build for android

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android
```

Build the cdylib for all the targets

```sh
cross build -p drop_core --target aarch64-linux-android
cross build -p drop_core --target armv7-linux-androideab
cross build -p drop_core --target i686-linux-android
cross build -p drop_core --target x86_64-linux-android
```

Generate the bindings using uniffi for kotlin

```sh
cargo run -p uniffi-bingen generate --library target/x86_64-linux-android/debug/libdrop_core.so --language=kotlin --out-dir ./bindings
```
