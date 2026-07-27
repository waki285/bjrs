# bjrs web

React + Vite demo for the `bjrs` engine compiled to `wasm32-unknown-unknown` via `wasm-bindgen`.

## Requirements

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-pack`
- Node.js + pnpm

## Quick start

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked

cd web
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` builds the wasm package into `web/wasm/pkg` and starts Vite.

## Cloudflare Pages deployment

`.github/workflows/deploy-pages.yml` builds and deploys `web/dist` after every push to `main`.

Before the first deployment:

1. Create a Cloudflare Pages Direct Upload project. The workflow sets `main` as its production branch.
2. Add these repository secrets:
   - `CLOUDFLARE_API_TOKEN`: a Cloudflare API token with `Account` / `Cloudflare Pages` / `Edit` permission.
   - `CLOUDFLARE_ACCOUNT_ID`: the Cloudflare account ID that owns the Pages project.
3. Add the `CLOUDFLARE_PAGES_PROJECT` repository variable with the exact Pages project name.
