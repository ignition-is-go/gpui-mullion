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
function activity(state, pane) {
  return state.activeActivities.find((entry) => entry.pane === pane)?.activity;
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
  const assert = (condition, message, state) => {
    if (!condition) throw new Error(`${message}: ${JSON.stringify(state)}`);
  };
  const action = async (name, payload = {}) => {
    const serialized = await evaluate(
      `globalThis.__mullionTestAction(${JSON.stringify(name)}, ${JSON.stringify(payload)})`,
    );
    if (typeof serialized !== "string") {
      throw new Error(`test action ${name} unavailable: ${JSON.stringify(serialized)}`);
    }
    return JSON.parse(serialized);
  };

  // Keep this sequence in lockstep with the canonical browser bridge installed
  // by examples/demo.rs. The bridge performs model actions; hover deliberately
  // uses CDP pointer input so the rendered GPUI activity bar is exercised too.
  let state = await action("reset");
  assert(state.activeWorkspace === "default", "reset selects Default", state);
  assert(state.tree.Split?.ratio === 0.4, "reset restores canonical 40/60 split", state);
  assert(state.catalog.primary[2].children[1].name === "Advanced", "nested category is exposed", state);
  assert(state.catalog.trailing[0].name === "Settings", "trailing Settings is exposed", state);

  state = await action("category", { category: "3" });
  assert(state.selectedCategory === "3" && activity(state, "1") === "11",
    "nested category selects Keybindings", state);

  // Pane 1 occupies the left 40% of the default workspace. Move outside first
  // so the test observes a genuine enter transition on its real vertical bar.
  await move(left + width - 2, top + height - 2);
  await move(left + 14, top + height / 2);
  state = await waitFor("real activity bar hover", (next) => next.barHover === "1");

  state = await action("activity", { pane: "1", activity: "9" });
  assert(activity(state, "1") === "9", "trailing Settings activity selects", state);
  state = await action("focusBehavior", { value: "hover" });
  assert(state.focusBehavior === "Hover", "Settings focus behavior updates", state);

  state = await action("palette", { query: "keybindings" });
  assert(state.paletteOpen && state.paletteQuery === "keybindings"
    && state.paletteResults.some((id) => id.endsWith(".11")),
  "palette finds nested Keybindings activity", state);

  for (const workspace of ["triple", "stacked", "default"]) {
    state = await action("workspace", { id: workspace });
    assert(state.activeWorkspace === workspace, `${workspace} workspace switches`, state);
  }
  state = await action("split", { pane: "1", direction: "horizontal" });
  assert(state.splitSequence === 1 && treeContains(state, "split-1"), "split factory runs", state);
  state = await action("drop", { activity: "5", destination: "2", edge: "bottom" });
  assert(state.dropSequence === 1 && treeContains(state, "drop-1"), "drop factory runs", state);

  state = await action("reset");
  assert(state.activeWorkspace === "default" && state.splitSequence === 0 && state.dropSequence === 0,
    "final reset restores sequences", state);
  assert(!state.paletteOpen && state.selectedCategory === null && state.barHover === null,
    "final reset clears transient controls", state);
  assert(!treeContains(state, "split-1") && !treeContains(state, "drop-1"),
    "final reset restores canonical tree", state);

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
