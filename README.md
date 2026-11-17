![Flip the Table](.github/banner.jpg)

# Flip the Table (╯°□°)╯︵ ┻━┻

[**Sick of work? Flip the table!**](https://flipthetable.alexkennedy.dev)

[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/Bevy-0.20%20WASM-232326?logo=bevy&logoColor=white)](https://bevyengine.org/)
[![Avian 3D](https://img.shields.io/badge/Avian3D-0.8%20Physics-ff6b35)](https://github.com/Jondolf/avian)
[![Next.js](https://img.shields.io/badge/Next.js-15.3-000000?logo=nextdotjs&logoColor=white)](https://nextjs.org/)
[![Cloudflare Pages](https://img.shields.io/badge/Cloudflare-Pages-F38020?logo=cloudflare&logoColor=white)](https://pages.cloudflare.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-22c55e)](LICENSE)

> [!WARNING]
> **Slop Alert:** I haven't even read this code.

A 3D retro pixel-art table-flipping game in the browser, built with Rust (Bevy + Avian3D) compiled to WebAssembly and wrapped in a Next.js frontend. Built by Gemini for funsies.

## Getting Started

### Prerequisites

- **Rust** (2024 edition) with the `wasm32-unknown-unknown` target
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/)
- **Node.js** 20+ and `npm`
- `gzip`

### Local Development

```bash
cd web
npm install
npm run dev
```

This builds `crates/game` in dev mode, compresses `flipthetable_game_bg.wasm` to `.wasm.gz`, and starts the Next.js dev server.

### Production Build

```bash
cd web
npm run build
```

## Deployment

Configured for **Cloudflare Pages** via [`web/wrangler.jsonc`](web/wrangler.jsonc):

```bash
cd web
npm run deploy
```

The login page and scoring is managed by Firebase.

## License

[MIT](LICENSE)
