# ARK Drop

ARK Drop is designed for easy file transfer. You can use QR codes to quickly send and receive files between devices. This app is part of ARK Framework and uses [`iroh`](https://iroh.computer/docs) to sync data between devices.

> [!WARNING]
> ARK Drop is currently under heavy development and should be used with caution. It has not undergone extensive testing and may contain bugs, vulnerabilities, or unexpected behavior.

## Development

ARK Drop is built using [Tauri](https://tauri.app/) with [SvelteKit](https://kit.svelte.dev/).

### Tauri

[Tauri](https://tauri.app/) is a framework for creating small, fast binaries for all major desktop platforms. It allows integration with any frontend framework that compiles to HTML, JavaScript, and CSS. Tauri prioritizes [security](https://tauri.app/v1/guides/development/security) and provides a detailed [architecture guide](https://tauri.app/v1/guides/architecture/).

### SvelteKit

[SvelteKit](https://kit.svelte.dev/) is an application framework built on Svelte. Unlike traditional frameworks, SvelteKit shifts work to a compile step during the build process, resulting in code that directly updates the DOM when the application's state changes, enhancing performance.

## Running ARK Drop Locally

You can use either `cargo tauri` CLI or `npm` CLI commands to run ARK Drop locally.

### Installing `cargo tauri`

To install `cargo tauri`, run:

```sh
cargo install tauri-cli
```

### Starting the Tauri Development Window

To start the Tauri development window, run:

```sh
npm run tauri dev
```

or

```sh
cargo tauri dev
```

This command builds the Rust code and opens the webview to display your web app. You can make changes to your web app, and if your tooling supports it, the webview will update automatically, similar to a browser.

## Building the Project

Tauri will detect your operating system and build a corresponding bundle. To build the project, run:

```sh
npm run tauri build
```

or

```sh
cargo tauri build
```

This process will build your frontend, compile the Rust binary, gather all external binaries and resources, and produce platform-specific bundles and installers.

For more information about Tauri builds, refer to the [Tauri building guide](https://tauri.app/v1/guides/building/).
