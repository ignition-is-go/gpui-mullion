#!/usr/bin/env node
import { writeFile } from "node:fs/promises";

const [url, port = "9222"] = process.argv.slice(2);
if (!url) throw new Error("usage: check_browser.mjs URL [DEBUG_PORT]");
const deadline = Date.now() + 60_000;
const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
let page;
while (Date.now() < deadline) {
  try {
    const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    page = pages.find((entry) => entry.type === "page" && entry.url.startsWith(url));
    if (page) break;
  } catch {}
  await sleep(250);
}
if (!page) throw new Error("Chrome did not open the requested page through DevTools");

const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let nextId = 0;
const pending = new Map();
socket.addEventListener("message", (event) => {
  const message = JSON.parse(event.data);
  if (!message.id) return;
  const waiter = pending.get(message.id);
  if (!waiter) return;
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
  const result = await command("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  if (result.exceptionDetails) throw new Error(`evaluation failed: ${JSON.stringify(result.exceptionDetails)}`);
  return result.result?.value;
}
async function runtimeState() {
  return evaluate(`(() => {
    const canvas = document.querySelector("canvas");
    const rect = canvas?.getBoundingClientRect();
    const expectedWidth = rect && Math.floor(rect.width * devicePixelRatio);
    const expectedHeight = rect && Math.floor(rect.height * devicePixelRatio);
    return {
      started: typeof globalThis.wasmBindings === "object",
      isolated: globalThis.crossOriginIsolated === true,
      bridge: typeof globalThis.__mullionTestState === "function",
      canvas: Boolean(canvas && canvas.isConnected && rect.width > 0 && rect.height > 0
        && canvas.width >= Math.max(2, expectedWidth * 0.9)
        && canvas.height >= Math.max(2, expectedHeight * 0.9)),
      rect: rect && { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
      href: location.href,
    };
  })()`);
}
async function testState() {
  const serialized = await evaluate(`globalThis.__mullionTestState?.()`);
  if (typeof serialized !== "string") throw new Error(`test bridge unavailable: ${JSON.stringify(serialized)}`);
  return JSON.parse(serialized);
}
async function waitFor(label, predicate, timeout = 8_000) {
  const until = Date.now() + timeout;
  let state;
  while (Date.now() < until) {
    state = await testState();
    if (predicate(state)) return state;
    await sleep(100);
  }
  throw new Error(`${label} timed out; state=${JSON.stringify(state)}`);
}
async function move(x, y) {
  await command("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none" });
}
async function click(x, y) {
  await move(x, y);
  await command("Input.dispatchMouseEvent", {
    type: "mousePressed", x, y, button: "left", buttons: 1, clickCount: 1,
  });
  await command("Input.dispatchMouseEvent", {
    type: "mouseReleased", x, y, button: "left", buttons: 0, clickCount: 1,
  });
}
async function drag(from, to) {
  await move(from.x, from.y);
  await command("Input.dispatchMouseEvent", {
    type: "mousePressed", x: from.x, y: from.y, button: "left", buttons: 1, clickCount: 1,
  });
  const steps = 24;
  for (let step = 1; step <= steps; step += 1) {
    const progress = step / steps;
    const x = from.x + (to.x - from.x) * progress;
    const y = from.y + (to.y - from.y) * progress;
    await command("Input.dispatchMouseEvent", {
      type: "mouseMoved", x, y, button: "left", buttons: 1,
    });
    await sleep(20);
  }
  await sleep(150);
  await command("Input.dispatchMouseEvent", {
    type: "mouseReleased", x: to.x, y: to.y, button: "left", buttons: 0, clickCount: 1,
  });
}
async function key({ key, code, keyCode, modifiers }) {
  const common = {
    key,
    code,
    windowsVirtualKeyCode: keyCode,
    nativeVirtualKeyCode: keyCode,
    modifiers,
  };
  await command("Input.dispatchKeyEvent", { type: "keyDown", ...common });
  await command("Input.dispatchKeyEvent", { type: "keyUp", ...common });
}
function paneState(state, pane) {
  return state.activeActivities.find((entry) => entry.pane === pane);
}
function activity(state, pane) {
  return paneState(state, pane)?.activity;
}
function treeContains(state, pane) {
  return JSON.stringify(state.tree).includes(`"${pane}"`);
}
async function captureFailure(error) {
  let runtime;
  let mullion;
  try { runtime = await runtimeState(); } catch (captureError) { runtime = { error: String(captureError) }; }
  try { mullion = await testState(); } catch (captureError) { mullion = { error: String(captureError) }; }
  const diagnostic = { error: String(error?.stack ?? error), runtime, mullion };
  console.error(JSON.stringify(diagnostic, null, 2));
  await writeFile("/tmp/mullion-browser-failure.json", `${JSON.stringify(diagnostic, null, 2)}\n`);
  try {
    const screenshot = await command("Page.captureScreenshot", { format: "png", fromSurface: true });
    await writeFile("/tmp/mullion-browser-failure.png", Buffer.from(screenshot.data, "base64"));
    console.error("diagnostic screenshot: /tmp/mullion-browser-failure.png");
  } catch (captureError) {
    console.error(`screenshot capture failed: ${captureError}`);
  }
}

await command("Page.enable");
await command("Runtime.enable");
try {
  let ready;
  let readySince;
  while (Date.now() < deadline) {
    ready = await runtimeState();
    if (ready.started && ready.isolated && ready.canvas && ready.bridge) {
      readySince ??= Date.now();
      // A dropped embedded Application briefly creates and then removes its canvas.
      if (Date.now() - readySince >= 3_000) break;
    } else {
      readySince = undefined;
    }
    await sleep(500);
  }
  if (!readySince || Date.now() - readySince < 3_000) {
    throw new Error(`GPUI browser readiness timed out: ${JSON.stringify(ready)}`);
  }

  const { left, top, width, height } = ready.rect;
  const workspaceHeight = 25;
  // The demo is a 62% horizontal split with a 50% vertical split on its right.
  const editor = { x: left + width * 0.31, y: top + workspaceHeight + (height - workspaceHeight) * 0.5 };
  const logs = { x: left + width * 0.81, y: top + workspaceHeight + (height - workspaceHeight) * 0.75 };

  let state = await testState();
  if (state.activeWorkspace !== "rship" || !treeContains(state, "editor")) {
    throw new Error(`unexpected initial demo state: ${JSON.stringify(state)}`);
  }

  await move(logs.x, logs.y);
  state = await waitFor("default hover focus", (next) => next.focused === "logs");

  await click(editor.x, editor.y);
  state = await waitFor("canvas pane click focus", (next) => next.focused === "editor");

  // The enabled split controls are the two compact buttons immediately above
  // Close at the bottom of the pinned rail. Stay clear of Close and scan only
  // that bounded strip so viewport height and font rasterization do not matter.
  const splitControlX = left + 14;
  for (let y = top + height - 96; y <= top + height - 60 && !treeContains(state, "split-1"); y += 4) {
    await click(splitControlX, y);
    await sleep(60);
    state = await testState();
  }
  if (!treeContains(state, "split-1") || !paneState(state, "split-1")?.project.startsWith("Split 1 from Rship")) {
    throw new Error(`visible split control did not create distinct split-1: ${JSON.stringify(state)}`);
  }

  // Select the real Logs activity item and drag that rendered DockDrag onto a
  // real pane zone. All input below is CDP pointer input; the bridge is read-only.
  const activityX = left + 14;
  const activityScanEnd = Math.min(top + height * 0.48, top + workspaceHeight + 150);
  let logsDragOrigin;
  for (let y = top + workspaceHeight + 6; y <= activityScanEnd; y += 6) {
    await click(activityX, y);
    await sleep(60);
    state = await testState();
    if (state.focused === "editor" && activity(state, "editor") === "logs") {
      logsDragOrigin = { x: activityX, y };
      break;
    }
  }
  if (!logsDragOrigin) {
    throw new Error(`logs activity control was not found: ${JSON.stringify(state)}`);
  }
  const terminalZone = {
    x: left + width * 0.72,
    y: top + workspaceHeight + (height - workspaceHeight) * 0.25,
  };
  await drag(logsDragOrigin, terminalZone);
  state = await waitFor("activity DockDrag drop", (next) =>
    treeContains(next, "drop-1")
      && activity(next, "drop-1") === "logs"
      && paneState(next, "drop-1")?.project === "Dropped logs #1 beside terminal");

  // Exercise registered Mullion actions through browser keyboard input, not a state hook.
  await key({ key: "End", code: "End", keyCode: 35, modifiers: 1 }); // Alt+End: FocusLast
  state = await waitFor("full-keymap focus action", (next) => next.focused === "logs");
  await key({ key: "Enter", code: "Enter", keyCode: 13, modifiers: 10 }); // Ctrl+Shift+Enter: ToggleZoom
  state = await waitFor("full-keymap zoom action", (next) => next.zoomed === "logs");
  // Keep the pane zoomed while switching workspaces; the switch must reconcile the
  // transient zoom because the destination workspace has different pane IDs.

  // Workspace switcher styling documents 12px horizontal padding; Browser is the second tab.
  await click(left + 90, top + 12);
  state = await waitFor("workspace tab click", (next) =>
    next.activeWorkspace === "browser"
      && treeContains(next, "browser-main")
      && treeContains(next, "browser-console")
      && next.zoomed === null);

  // Drive the real pinned-left activity rail. Browser/driver viewport geometry
  // varies (including very wide, short Xvfb canvases), so scan the narrow rail
  // rather than encoding one font/line-height-dependent y coordinate.
  const browserActivityX = left + 14;
  const browserActivityScanEnd = Math.min(top + height * 0.48, top + workspaceHeight + 150);
  let selectedTerminal = false;
  for (let y = top + workspaceHeight + 6; y <= browserActivityScanEnd; y += 6) {
    await click(browserActivityX, y);
    await sleep(75);
    state = await testState();
    if (state.focused === "browser-main" && activity(state, "browser-main") === "terminal") {
      selectedTerminal = true;
      break;
    }
  }
  if (!selectedTerminal) {
    throw new Error(`activity-bar item click timed out; state=${JSON.stringify(state)}`);
  }

  const liveSince = Date.now();
  while (Date.now() - liveSince < 2_000) {
    const live = await runtimeState();
    if (!live.canvas || !live.bridge) throw new Error(`canvas stopped after interaction: ${JSON.stringify(live)}`);
    await sleep(250);
  }
  console.log(JSON.stringify({ ...await runtimeState(), state, sustainedMs: Date.now() - readySince }));
} catch (error) {
  await captureFailure(error);
  socket.close();
  process.exitCode = 1;
}
if (process.exitCode !== 1) socket.close();
