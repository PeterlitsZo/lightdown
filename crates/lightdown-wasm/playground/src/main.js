import "./style.css";

import * as monaco from "monaco-editor";
import init, { renderToHtml } from "../../pkg/lightdown_wasm.js";

const SAMPLE_INPUT = `# Foobar

## Barfoo

Do you know \\(a {:href "https://example.com"} [\`lightdown\`])? \`lightdown\` is good.`;

const sourceEditor = document.querySelector("#sourceEditor");
const statusBadge = document.querySelector("#statusBadge");
const resultBadge = document.querySelector("#resultBadge");
const outputCode = document.querySelector("#outputCode");
const previewPanel = document.querySelector("#previewPanel");
const tabs = document.querySelectorAll("[data-tab]");
const panels = {
  output: document.querySelector("#outputPanel"),
  preview: previewPanel,
};

let activeTab = "preview";
const editor = monaco.editor.create(sourceEditor, {
  automaticLayout: true,
  language: "plaintext",
  minimap: { enabled: false },
  readOnly: true,
  scrollBeyondLastLine: false,
  tabSize: 2,
  theme: "vs",
  value: SAMPLE_INPUT,
});

for (const tab of tabs) {
  tab.addEventListener("click", () => {
    setActiveTab(tab.dataset.tab);
  });
}

editor.onDidChangeModelContent(() => {
  renderSource();
});

boot();

async function boot() {
  try {
    await init();
    editor.updateOptions({ readOnly: false });
    setBadge(statusBadge, "ready", "Wasm ready");
    renderSource();
    editor.focus();
  } catch (error) {
    const message = describeError(error);
    setBadge(statusBadge, "error", "Wasm failed");
    showRenderError(`Failed to initialize wasm.\n\n${message}`);
  }
}

function renderSource() {
  try {
    const html = renderToHtml(editor.getValue());
    outputCode.textContent = html;
    previewPanel.innerHTML = html;
    setBadge(resultBadge, "success", "Rendered");
  } catch (error) {
    showRenderError(describeError(error));
  }
}

function showRenderError(message) {
  outputCode.textContent = `Render failed\n\n${message}`;
  previewPanel.replaceChildren(createErrorMessage(message));
  setBadge(resultBadge, "error", "Render failed");
}

function createErrorMessage(message) {
  const wrapper = document.createElement("div");
  wrapper.className = "preview-error";

  const title = document.createElement("p");
  title.className = "preview-error-title";
  title.textContent = "Render failed";

  const body = document.createElement("pre");
  body.className = "preview-error-body";
  body.textContent = message;

  wrapper.append(title, body);
  return wrapper;
}

function setActiveTab(nextTab) {
  activeTab = nextTab;

  for (const tab of tabs) {
    const selected = tab.dataset.tab === nextTab;
    tab.classList.toggle("is-active", selected);
    tab.setAttribute("aria-selected", String(selected));
  }

  for (const [name, panel] of Object.entries(panels)) {
    const selected = name === nextTab;
    panel.classList.toggle("is-active", selected);
    panel.hidden = !selected;
  }
}

function setBadge(element, state, text) {
  if (!element) {
    return;
  }

  element.dataset.state = state;
  element.textContent = text;
}

function describeError(error) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

setActiveTab(activeTab);
