# Monaco Playground Design

## Goal

Replace the `lightdown-wasm` playground's left-side plain textarea with Monaco Editor, while keeping the current render pipeline, layout structure, and preview/output tabs unchanged.

## Scope

This change only covers the playground in `crates/lightdown-wasm/playground`.

Included:

- replace the source editor widget with Monaco Editor;
- keep the current sample content and live render behavior;
- keep the current Wasm boot flow;
- keep the current preview and output panes;
- keep the current visual layout with only the CSS changes required to host Monaco.

Excluded:

- custom Lightdown or Lightdown IR syntax highlighting;
- diagnostics, autocomplete, formatting, or hover support;
- broader UI redesign of the playground;
- editor persistence or URL state sync.

## Approach

Use the `monaco-editor` npm package directly in the Vite playground build.

The HTML editor pane will stop rendering a `<textarea>` and instead render a dedicated container element for Monaco. The JavaScript entrypoint will create one Monaco editor instance after the page loads, seed it with the existing sample input, and subscribe to model content changes to trigger the existing `renderSource` flow.

The editor will start in read-only mode until the Wasm module finishes initializing. After initialization succeeds, the editor will switch to editable mode, render the current content, and receive focus. If initialization fails, the result pane behavior will remain the same as today.

## Integration Details

### HTML

`index.html` will replace:

- `<textarea id="sourceInput">`

with:

- `<div id="sourceEditor"></div>`

No other structural layout change is required.

### JavaScript

`src/main.js` will:

- import `monaco-editor`;
- create a single editor instance bound to `#sourceEditor`;
- set the editor value to the current sample input;
- read source text via `editor.getValue()` instead of `textarea.value`;
- listen to editor content changes instead of textarea `input` events;
- toggle read-only mode through `editor.updateOptions({ readOnly: ... })`;
- focus the editor after Wasm initialization succeeds.

The rendering code path will stay otherwise unchanged:

- `renderToHtml(...)`
- HTML output panel update
- preview panel update
- status badge update

### CSS

`src/style.css` will:

- remove textarea-specific styling;
- make the editor container fill the editor pane;
- ensure the Monaco editor root can stretch to the full pane height.

No palette or layout redesign is planned.

### Dependencies

`playground/package.json` will add:

- `monaco-editor`

No additional loader package is needed.

## Risks

### Bundle size

Monaco will increase the playground bundle size. This is acceptable for the current goal because the playground is a development-facing tool.

### Layout sizing

Monaco requires a stable container size. The CSS changes must ensure the editor pane and editor container have a definite height so the editor can render correctly.

### Event frequency

Monaco content change events may fire often. For this change, the current direct render-on-change behavior will be preserved because that matches the existing textarea experience and keeps scope small.

## Verification

- `npm install --prefix crates/lightdown-wasm/playground`
- `npm run build --prefix crates/lightdown-wasm/playground`
- if needed, `cargo test`

The build should succeed, and the playground should still render sample Lightdown input to both preview and output panes.
