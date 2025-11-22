# ARK Drop

ARK Drop is designed for easy file transfer. You can use QR codes to quickly send and receive files between devices. This app is part of ARK Framework and uses [`iroh`](https://iroh.computer/docs) to sync data between devices.

> [!WARNING]
> ARK Drop is currently under heavy development and should be used with caution. It has not undergone extensive testing and may contain bugs, vulnerabilities, or unexpected behavior.

## Tech Stack

ARK Drop is built using [Tauri](https://tauri.app/) with [SvelteKit](https://kit.svelte.dev/).

### Tauri

[Tauri](https://tauri.app/) is a framework for creating small, fast binaries for all major desktop platforms. It allows integration with any frontend framework that compiles to HTML, JavaScript, and CSS. Tauri prioritizes [security](https://tauri.app/v1/guides/development/security) and provides a detailed [architecture guide](https://tauri.app/v1/guides/architecture/).

### SvelteKit

[SvelteKit](https://kit.svelte.dev/) is an application framework built on Svelte. Unlike traditional frameworks, SvelteKit shifts work to a compile step during the build process, resulting in code that directly updates the DOM when the application's state changes, enhancing performance.

## Development

### Prerequisites

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/)

### Install Dependencies

```sh
npm install
```

### Run Development Server

```sh
npm run tauri dev
```

This command builds the Rust code and opens the webview to display your web app. You can make changes to your web app, and if your tooling supports it, the webview will update automatically, similar to a browser.

## Build

Tauri will detect your operating system and build a corresponding bundle.

```sh
npm run tauri build
```

This process will build your frontend, compile the Rust binary, gather all external binaries and resources, and produce platform-specific bundles and installers.

For more information, refer to the [Tauri building guide](https://tauri.app/v1/guides/building/).

## Android Build

Cross-compilation is easiest using [Cross](https://github.com/cross-rs/cross). Alternatively, you can set up the NDK and build manually.

### Add Android Targets

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### Build for Android Targets

Using Cross:
```sh
cross build -p drop_core --target aarch64-linux-android
cross build -p drop_core --target armv7-linux-androideabi
cross build -p drop_core --target i686-linux-android
cross build -p drop_core --target x86_64-linux-android
```

Or using cargo-ndk:
```sh
cargo install cargo-ndk
cargo ndk -o ./target/release/jniLibs --target aarch64-linux-android --target armv7-linux-androideabi --target i686-linux-android --target x86_64-linux-android build -p drop_core --release
```
