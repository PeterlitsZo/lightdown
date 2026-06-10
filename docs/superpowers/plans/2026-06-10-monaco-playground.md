# Monaco Playground Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `lightdown-wasm` playground textarea with Monaco Editor while keeping the current live render workflow unchanged.

**Architecture:** The playground will keep the same two-pane layout and the same wasm-backed render path. The left pane will render a Monaco host element instead of a textarea, and the JavaScript entrypoint will own a single Monaco editor instance that feeds the existing preview and source output panes.

**Tech Stack:** Vite, JavaScript, CSS, `monaco-editor`, `lightdown-wasm`.

---

### Task 1: Swap The Editor Host

**Files:**
- Modify: `crates/lightdown-wasm/playground/index.html`
- Modify: `crates/lightdown-wasm/playground/src/style.css`

- [ ] Replace the textarea node with a dedicated Monaco container.
- [ ] Remove textarea-specific styling and make the editor host fill the pane height.

### Task 2: Wire Monaco Into The Playground

**Files:**
- Modify: `crates/lightdown-wasm/playground/src/main.js`

- [ ] Import `monaco-editor` directly in the Vite entrypoint.
- [ ] Create one editor instance with the current sample input.
- [ ] Preserve the current wasm initialization flow, render-on-change behavior, badges, and focus behavior using Monaco APIs.

### Task 3: Add The Dependency And Verify

**Files:**
- Modify: `crates/lightdown-wasm/playground/package.json`
- Modify: `crates/lightdown-wasm/playground/package-lock.json`

- [ ] Add `monaco-editor` to the playground dependencies.
- [ ] Refresh the lockfile with npm.
- [ ] Verify the change with `npm run build --prefix crates/lightdown-wasm/playground`.
