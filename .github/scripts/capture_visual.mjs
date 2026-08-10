#!/usr/bin/env node
// Dependency-free Chrome DevTools Protocol screenshot driver.
import { mkdir, writeFile } from "node:fs/promises";

const [url, output, adapter, port = "9223"] = process.argv.slice(2);
if (!url || !output || !["reference", "gpui"].includes(adapter)) {
  throw new Error("usage: capture_visual.mjs URL OUTPUT_DIR reference|gpui [DEBUG_PORT]");
}
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
await mkdir(output, { recursive: true });
let page;
for (let attempt = 0; attempt < 120; attempt += 1) {
  try {
    // Each app owns a distinct page. Reusing the reference page for GPUI made
    // Chrome tear down one document while creating its WebGPU swapchain in the
    // next, which is particularly fragile with SwiftShader under Xvfb.
    const response = await fetch(`http://127.0.0.1:${port}/json/new?about%3Ablank`, { method: "PUT" });
    if (response.ok) page = await response.json();
    if (page?.type === "page") break;
  } catch {}
  await sleep(250);
}
if (!page) throw new Error("Chrome DevTools could not create an isolated capture page");
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let nextId = 0;
const pending = new Map();
socket.addEventListener("message", (event) => {
  const message = JSON.parse(event.data);
  if (!message.id || !pending.has(message.id)) return;
  const waiter = pending.get(message.id);
  pending.delete(message.id);
  if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
  else waiter.resolve(message.result);
});
function command(method, params = {}) {
  const id = ++nextId;
  socket.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
}
async function evaluate(expression) {
  const result = await command("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result?.value;
}
async function ready() {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    const state = await evaluate(`({complete:document.readyState === "complete", fonts:document.fonts.status,
      visible:document.visibilityState === "visible", canvas:!!document.querySelector("canvas"),
      body:document.body?.getBoundingClientRect().width || 0,
      bridge:typeof globalThis.__mullionTestState === "function"})`);
    if (state.complete && state.fonts === "loaded" && state.visible && state.body > 0
        && (adapter === "reference" || (state.canvas && state.bridge))) {
      await evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))");
      await sleep(500);
      return;
    }
    await sleep(200);
  }
  throw new Error(`${adapter} readiness timeout at ${url}`);
}
let navigation = 0;
async function reset(width, height) {
  await command("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await command("Emulation.setFocusEmulationEnabled", { enabled: true });
  await command("Page.bringToFront");
  await command("Storage.clearDataForOrigin", { origin: new URL(url).origin, storageTypes: "all" });
  const captureUrl = new URL(url);
  captureUrl.searchParams.set("mullionVisual", `${adapter}-${++navigation}`);
  await command("Page.navigate", { url: captureUrl.href });
  await ready();
  if (adapter === "gpui") await bridgeAction("reset");
}
async function mouse(x, y, type = "mouseMoved") {
  await command("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" ? "none" : "left",
    buttons: type === "mousePressed" ? 1 : 0, clickCount: type === "mouseMoved" ? 0 : 1 });
}
async function click(x, y) {
  await mouse(x, y); await mouse(x, y, "mousePressed"); await mouse(x, y, "mouseReleased"); await sleep(400);
}
async function bridgeAction(name, payload = {}) {
  if (adapter !== "gpui") throw new Error(`GPUI bridge action ${name} requested for reference app`);
  const value = await evaluate(
    `globalThis.__mullionTestAction(${JSON.stringify(name)}, ${JSON.stringify(payload)})`,
  );
  if (typeof value !== "string") throw new Error(`GPUI bridge action ${name} was unavailable`);
  await evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  await sleep(300);
  return JSON.parse(value);
}
async function bridgeState() {
  const value = await evaluate("globalThis.__mullionTestState()");
  if (typeof value !== "string") throw new Error("GPUI bridge state was unavailable");
  return JSON.parse(value);
}
async function center(selector, index = 0) {
  const point = await evaluate(`(() => { const r = document.querySelectorAll(${JSON.stringify(selector)})[${index}]?.getBoundingClientRect();
    return r && {x:r.left+r.width/2,y:r.top+r.height/2}; })()`);
  if (!point) throw new Error(`missing deterministic reference control: ${selector}[${index}]`);
  return point;
}
async function key(keyName, code, keyCode, modifiers) {
  const common = { key: keyName, code, windowsVirtualKeyCode: keyCode, nativeVirtualKeyCode: keyCode, modifiers };
  await command("Input.dispatchKeyEvent", { type: "keyDown", ...common });
  await command("Input.dispatchKeyEvent", { type: "keyUp", ...common });
  await sleep(500);
}
async function shot(name, width, height, action) {
  await reset(width, height);
  // Let both hosts settle viewport geometry before applying a pointer-driven
  // scenario; resizing afterward synthesizes a GPUI hover leave and collapses
  // the very state the screenshot is meant to prove.
  await evaluate("window.dispatchEvent(new Event('resize')); new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  await command("Page.bringToFront");
  if (action) await action(width, height);
  await sleep(250);
  const png = await command("Page.captureScreenshot", { format: "png", fromSurface: true,
    captureBeyondViewport: false, clip: { x: 0, y: 0, width, height, scale: 1 } });
  await writeFile(`${output}/${name}.png`, Buffer.from(png.data, "base64"));
  console.log(`${adapter}: ${name} (${width}x${height})`);
}
await command("Page.enable");
await command("Runtime.enable");
const scenarios = [
  ["initial-nested-3-panes-1000x700", 1000, 700],
  ["initial-nested-3-panes-1280x720", 1280, 720],
  ["initial-nested-3-panes-800x600", 800, 600],
  ["initial-nested-3-panes-390x844", 390, 844],
  ["vertical-rail-hovered-expanded-1280x720", 1280, 720, async () => {
    if (adapter === "reference") {
      const point = await center(".mullion-ab"); await mouse(point.x, point.y);
    } else {
      await bridgeAction("barHover", { pane: "1" });
      // Keep the real browser pointer inside the resolved panel. Otherwise the
      // canvas correctly delivers a leave after the bridge's synthetic entry.
      await mouse(74, 43);
      const state = await bridgeState();
      if (state.barHover !== "1") throw new Error("GPUI rail did not remain expanded");
    }
    await sleep(900);
  }],
  ["category-card-open-1280x720", 1280, 720, async () => {
    if (adapter === "reference") {
      const rail = await center(".mullion-ab"); await mouse(rail.x, rail.y); await sleep(700);
      const category = await center(".mullion-ab-category .mullion-ab-btn"); await click(category.x, category.y);
    } else {
      await bridgeAction("barHover", { pane: "1" });
      await mouse(74, 43);
      let state = await bridgeState();
      if (state.barHover !== "1") throw new Error("GPUI rail did not remain expanded");
      await sleep(700);
      // Drive the same catalog category as the reference selector. A canvas
      // coordinate can instead hit the already-revealed card and collapse it.
      state = await bridgeAction("category", { category: "0" });
      const pane = state.activeActivities.find(({ pane }) => pane === "1");
      if (state.selectedCategory !== "0" || pane?.activity !== "1") {
        throw new Error(`GPUI category did not resolve expected model state: ${JSON.stringify(state)}`);
      }
    }
    await sleep(500);
  }],
  ["focus-unfocused-wash-1280x720", 1280, 720, async () => {
    if (adapter === "reference") {
      const point = await center(".mullion-pane", 1); await click(point.x, point.y);
    } else {
      await bridgeAction("focus", { pane: "2" });
    }
  }],
  ["command-palette-overlay-1280x720", 1280, 720, async () => {
    if (adapter === "reference") await key("k", "KeyK", 75, 2);
    else await bridgeAction("palette", { query: "" });
  }],
  ["workspace-switch-1280x720", 1280, 720, async (_width, height) => {
    if (adapter === "reference") { const point = await center(".demo-layout-footer button", 1); await click(point.x, point.y); }
    else await bridgeAction("workspace", { id: "triple" });
  }],
];
for (const scenario of scenarios) await shot(...scenario);
await writeFile(`${output}/capture.json`, `${JSON.stringify({ adapter, url, scenarios: scenarios.map(([name,w,h]) => ({name,width:w,height:h})),
  chrome: await evaluate("navigator.userAgent"), devicePixelRatio: await evaluate("devicePixelRatio"),
  gpuAdapter: await evaluate(`navigator.gpu?.requestAdapter().then(adapter => adapter && ({
    vendor: adapter.info.vendor, architecture: adapter.info.architecture,
    device: adapter.info.device, description: adapter.info.description,
  }))`),
  canvas: await evaluate(`(() => { const canvas = document.querySelector("canvas");
    const rect = canvas?.getBoundingClientRect(); return canvas && {
      width: canvas.width, height: canvas.height,
      rect: {x: rect.x, y: rect.y, width: rect.width, height: rect.height},
    }; })()`),
}, null, 2)}\n`);
socket.close();
