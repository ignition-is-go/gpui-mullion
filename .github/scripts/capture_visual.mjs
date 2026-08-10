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
    const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    page = pages.find((entry) => entry.type === "page");
    if (page) break;
  } catch {}
  await sleep(250);
}
if (!page) throw new Error("Chrome DevTools page was not available");
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
      canvas:!!document.querySelector("canvas"), body:document.body?.getBoundingClientRect().width || 0,
      bridge:typeof globalThis.__mullionTestState === "function"})`);
    if (state.complete && state.fonts === "loaded" && state.body > 0
        && (adapter === "reference" || (state.canvas && state.bridge))) {
      await evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))");
      await sleep(500);
      return;
    }
    await sleep(200);
  }
  throw new Error(`${adapter} readiness timeout at ${url}`);
}
async function reset(width, height) {
  await command("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await command("Page.navigate", { url });
  await ready();
  await evaluate("localStorage.clear(); sessionStorage.clear();");
  await command("Page.reload", { ignoreCache: true });
  await ready();
}
async function mouse(x, y, type = "mouseMoved") {
  await command("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" ? "none" : "left",
    buttons: type === "mousePressed" ? 1 : 0, clickCount: type === "mouseMoved" ? 0 : 1 });
}
async function click(x, y) {
  await mouse(x, y); await mouse(x, y, "mousePressed"); await mouse(x, y, "mouseReleased"); await sleep(400);
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
  if (action) await action(width, height);
  const png = await command("Page.captureScreenshot", { format: "png", fromSurface: true,
    captureBeyondViewport: false, clip: { x: 0, y: 0, width, height, scale: 1 } });
  await writeFile(`${output}/${name}.png`, Buffer.from(png.data, "base64"));
  console.log(`${adapter}: ${name} (${width}x${height})`);
}
await command("Page.enable");
await command("Runtime.enable");
const scenarios = [
  ["initial-nested-3-panes-1280x720", 1280, 720],
  ["initial-nested-3-panes-960x600", 960, 600],
  ["vertical-rail-hovered-expanded-1280x720", 1280, 720, async () => {
    const point = adapter === "reference" ? await center(".mullion-ab") : {x: 18, y: 92};
    await mouse(point.x, point.y); await sleep(900);
  }],
  ["category-card-open-1280x720", 1280, 720, async () => {
    if (adapter === "reference") {
      const rail = await center(".mullion-ab"); await mouse(rail.x, rail.y); await sleep(700);
      const category = await center(".mullion-ab-category .mullion-ab-btn"); await click(category.x, category.y);
    } else {
      await mouse(18, 68); await sleep(700); await click(76, 68);
    }
    await sleep(500);
  }],
  ["focus-unfocused-wash-1280x720", 1280, 720, async () => {
    const point = adapter === "reference" ? await center(".mullion-pane", 1) : {x: 930, y: 250};
    await click(point.x, point.y);
  }],
  ["command-palette-overlay-1280x720", 1280, 720, async () => { await key("k", "KeyK", 75, 2); }],
  ["workspace-switch-1280x720", 1280, 720, async (_width, height) => {
    if (adapter === "reference") { const point = await center(".demo-layout-footer button", 1); await click(point.x, point.y); }
    else await click(90, 12);
  }],
];
for (const scenario of scenarios) await shot(...scenario);
await writeFile(`${output}/capture.json`, `${JSON.stringify({ adapter, url, scenarios: scenarios.map(([name,w,h]) => ({name,width:w,height:h})),
  chrome: await evaluate("navigator.userAgent"), devicePixelRatio: await evaluate("devicePixelRatio") }, null, 2)}\n`);
socket.close();
