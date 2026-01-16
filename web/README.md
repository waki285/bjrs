# bjrs web

React + Vite demo for the `bjrs` engine compiled to `wasm32-unknown-unknown` via `wasm-bindgen`.

## Requirements

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-pack`
- Node.js + npm

## Quick start

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

cd web
npm install
npm run dev
```

`npm run dev` builds the wasm package into `web/wasm/pkg` and starts Vite.
