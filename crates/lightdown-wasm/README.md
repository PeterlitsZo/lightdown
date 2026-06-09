# lightdown-wasm

WebAssembly bindings for Lightdown.

## Build

```sh
wasm-pack build crates/lightdown-wasm --target web --out-dir pkg
```

The generated npm package will be written to `crates/lightdown-wasm/pkg`.

## Playground

The crate ships with a Vite playground in `crates/lightdown-wasm/playground`.

Build the wasm package first:

```sh
wasm-pack build crates/lightdown-wasm --target web --out-dir pkg
```

Install the playground dependencies:

```sh
npm install --prefix crates/lightdown-wasm/playground
```

Start the dev server:

```sh
npm run dev --prefix crates/lightdown-wasm/playground
```

Build the static playground assets:

```sh
npm run build --prefix crates/lightdown-wasm/playground
```

If you change the Rust code, rebuild `pkg` before refreshing the playground.

## Publish

```sh
wasm-pack build crates/lightdown-wasm --target web --out-dir pkg --release
npm publish crates/lightdown-wasm/pkg
```

## Usage

```js
import init, { renderToHtml } from "./pkg/lightdown_wasm.js";

await init();

const html = renderToHtml(`(doc
  {:meta {:version "0.1.0"}}
  (p "Hello"))`);
```
