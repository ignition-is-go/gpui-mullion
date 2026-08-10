#!/usr/bin/env node
// Deterministic smoke test for a separately launched `trunk serve` demo.
// Usage: node examples/web/check_demo_bridge.mjs http://127.0.0.1:8080 [9222]
const [url, port = "9222"] = process.argv.slice(2);
if (!url) throw new Error("usage: check_demo_bridge.mjs URL [DEBUG_PORT]");
const deadline = Date.now() + 30_000;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let page;
while (!page && Date.now() < deadline) {
  try {
    const pages = await (
      await fetch(`http://127.0.0.1:${port}/json/list`)
    ).json();
    page = pages.find(
      (candidate) => candidate.type === "page" && candidate.url.startsWith(url),
    );
  } catch {}
  if (!page) await sleep(200);
}
if (!page) throw new Error("demo page not found in Chrome DevTools");
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let sequence = 0;
const pending = new Map();
socket.addEventListener("message", ({ data }) => {
  const message = JSON.parse(data);
  const waiter = pending.get(message.id);
  if (!waiter) return;
  pending.delete(message.id);
  message.error ? waiter.reject(message.error) : waiter.resolve(message.result);
});
function command(method, params) {
  const id = ++sequence;
  socket.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
}
async function evaluate(expression) {
  const result = await command("Runtime.evaluate", {
    expression,
    returnByValue: true,
  });
  if (result.exceptionDetails)
    throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function action(name, payload = {}) {
  return JSON.parse(
    await evaluate(
      `globalThis.__mullionTestAction(${JSON.stringify(name)}, ${JSON.stringify(payload)})`,
    ),
  );
}
const assert = (condition, message, state) => {
  if (!condition) throw new Error(`${message}: ${JSON.stringify(state)}`);
};
while (
  Date.now() < deadline &&
  !(await evaluate("typeof globalThis.__mullionTestAction === 'function'"))
) {
  await sleep(100);
}
let state = await action("reset");
assert(state.activeWorkspace === "default", "reset selects Default", state);
assert(
  state.tree.Split.ratio === 0.4,
  "reset restores canonical 40/60 split",
  state,
);
assert(
  state.catalog.primary[2].children[1].name === "Advanced",
  "nested category is exposed",
  state,
);
assert(
  state.catalog.trailing[0].name === "Settings",
  "trailing Settings is exposed",
  state,
);
state = await action("category", { category: "3" });
assert(
  state.selectedCategory === "3" &&
    state.activeActivities.find(({ pane }) => pane === "1").activity === "11",
  "nested category fixture selects its canonical child",
  state,
);
const canvasRect = await evaluate(
  `(() => { const r = document.querySelector("canvas").getBoundingClientRect(); return JSON.stringify({left:r.left,top:r.top,width:r.width,height:r.height}); })()`,
);
const canvasOrigin = JSON.parse(canvasRect);
await command("Input.dispatchMouseEvent", {
  type: "mouseMoved",
  x: canvasOrigin.left + 14,
  y: canvasOrigin.top + 180,
  button: "none",
});
for (let attempt = 0; attempt < 40; attempt += 1) {
  state = JSON.parse(await evaluate("globalThis.__mullionTestState()"));
  if (state.barHover === "1") break;
  await sleep(50);
}
assert(state.barHover === "1", "live canvas hover expands the bar", state);
state = await action("activity", { pane: "1", activity: "9" });
assert(
  state.activeActivities.find(({ pane }) => pane === "1").activity === "9",
  "Settings selects",
  state,
);
state = await action("focusBehavior", { value: "hover" });
assert(state.focusBehavior === "Hover", "controlled setting updates", state);
state = await action("palette", { query: "keybindings" });
assert(
  state.paletteOpen &&
    state.paletteQuery === "keybindings" &&
    state.paletteResults.some((id) => id.endsWith(".11")),
  "real palette finds nested activity",
  state,
);
state = await action("paletteClose");
assert(
  !state.paletteOpen &&
    state.paletteQuery === "" &&
    state.paletteResults.length > 0,
  "real palette closes and clears its query",
  state,
);
state = await action("workspace", { id: "triple" });
assert(state.activeWorkspace === "triple", "Triple workspace switches", state);
state = await action("workspace", { id: "stacked" });
assert(
  state.activeWorkspace === "stacked",
  "Stacked workspace switches",
  state,
);
state = await action("workspace", { id: "default" });
for (const edge of ["Left", "Right", "Top", "Bottom"]) {
  state = await action("activityBar", { edge });
  assert(
    state.activityBar.edge === edge,
    `real activity bar moves to ${edge}`,
    state,
  );
}
for (const mode of ["Pinned", "AutoHide", "Hidden"]) {
  state = await action("activityBar", { mode });
  assert(
    state.activityBar.mode === mode,
    `real activity bar enters ${mode}`,
    state,
  );
}
state = await action("reset");
assert(
  state.activityBar.edge === "Left" && state.activityBar.mode === "Pinned",
  "reset restores activity bar policy",
  state,
);

state = await action("modal");
assert(
  state.overlays.length === 1 &&
    state.overlays[0].id === "modal" &&
    state.overlays[0].tier === "modal" &&
    state.overlays[0].dismiss_on_backdrop,
  "controlled modal exposes real backdrop policy",
  state,
);
// Controlled policy is visible synchronously; wait for the next GPUI paint so
// real CDP input targets the newly installed backdrop rather than stale hit data.
await sleep(300);
await command("Input.dispatchMouseEvent", {
  type: "mousePressed",
  x: canvasOrigin.left + 4,
  y: canvasOrigin.top + 4,
  button: "left",
  buttons: 1,
  clickCount: 1,
});
await command("Input.dispatchMouseEvent", {
  type: "mouseReleased",
  x: canvasOrigin.left + 4,
  y: canvasOrigin.top + 4,
  button: "left",
  buttons: 0,
  clickCount: 1,
});
for (let attempt = 0; attempt < 20; attempt += 1) {
  state = JSON.parse(await evaluate("globalThis.__mullionTestState()"));
  if (state.overlays.length === 0) break;
  await sleep(25);
}
assert(
  state.overlays.length === 0,
  "real backdrop click dismisses controlled modal",
  state,
);
state = await action("toast");
assert(
  state.overlays.length === 1 &&
    state.overlays[0].id === "toast" &&
    state.overlays[0].tier === "toast",
  "controlled toast is real stack state",
  state,
);
state = await action("focus", { pane: "2" });
state = await action("drag");
assert(
  state.overlays.length === 1 &&
    state.overlays[0].id === "drag" &&
    state.overlays[0].tier === "drag" &&
    state.overlays[0].click_through,
  "controlled drag overlay is click-through",
  state,
);
await sleep(300);
const paneOnePoint = {
  x: canvasOrigin.left + canvasOrigin.width * 0.25,
  y: canvasOrigin.top + canvasOrigin.height * 0.3,
};
await command("Input.dispatchMouseEvent", {
  type: "mousePressed",
  ...paneOnePoint,
  button: "left",
  buttons: 1,
  clickCount: 1,
});
await command("Input.dispatchMouseEvent", {
  type: "mouseReleased",
  ...paneOnePoint,
  button: "left",
  buttons: 0,
  clickCount: 1,
});
for (let attempt = 0; attempt < 20; attempt += 1) {
  state = JSON.parse(await evaluate("globalThis.__mullionTestState()"));
  if (state.focused === "1") break;
  await sleep(25);
}
assert(state.focused === "1", "click-through drag preserves pane input", state);
state = await action("all");
assert(
  state.overlays.map(({ id }) => id).join(",") === "modal,toast,drag",
  "all overlay tiers share one controlled stack",
  state,
);
state = await action("clear");
assert(
  state.overlays.length === 0,
  "clear empties controlled overlay stack",
  state,
);

state = await action("split", { pane: "1", direction: "horizontal" });
state = await action("drop", {
  activity: "5",
  destination: "2",
  edge: "bottom",
});
assert(
  JSON.stringify(state.tree).includes("split-1") &&
    JSON.stringify(state.tree).includes("drop-1"),
  "deterministic factories run",
  state,
);
state = await action("reset");
assert(
  state.splitSequence === 0 &&
    state.dropSequence === 0 &&
    !state.paletteOpen &&
    state.overlays.length === 0 &&
    state.activityBar.edge === "Left" &&
    state.activityBar.mode === "Pinned",
  "reset clears all bridge state",
  state,
);
assert(
  !JSON.stringify(state.tree).includes("split-1") &&
    !JSON.stringify(state.tree).includes("drop-1"),
  "reset restores trees",
  state,
);
console.log(JSON.stringify({ ok: true, state }));
socket.close();
