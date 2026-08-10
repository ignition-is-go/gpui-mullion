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
    const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    page = pages.find((candidate) => candidate.type === "page" && candidate.url.startsWith(url));
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
  const result = await command("Runtime.evaluate", { expression, returnByValue: true });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function action(name, payload = {}) {
  return JSON.parse(await evaluate(`globalThis.__mullionTestAction(${JSON.stringify(name)}, ${JSON.stringify(payload)})`));
}
const assert = (condition, message, state) => {
  if (!condition) throw new Error(`${message}: ${JSON.stringify(state)}`);
};
while (Date.now() < deadline && !(await evaluate("typeof globalThis.__mullionTestAction === 'function'"))) {
  await sleep(100);
}
let state = await action("reset");
assert(state.activeWorkspace === "default", "reset selects Default", state);
assert(state.tree.Split.ratio === 0.4, "reset restores canonical 40/60 split", state);
assert(state.catalog.primary[2].children[1].name === "Advanced", "nested category is exposed", state);
assert(state.catalog.trailing[0].name === "Settings", "trailing Settings is exposed", state);
state = await action("category", { category: "3" });
assert(
  state.selectedCategory === "3" && state.activeActivities.find(({ pane }) => pane === "1").activity === "11",
  "nested category fixture selects its canonical child",
  state,
);
state = await action("barHover", { pane: "1" });
assert(state.barHover === "1", "bar hover state is deterministic", state);
state = await action("activity", { pane: "1", activity: "9" });
assert(state.activeActivities.find(({ pane }) => pane === "1").activity === "9", "Settings selects", state);
state = await action("focusBehavior", { value: "hover" });
assert(state.focusBehavior === "Hover", "controlled setting updates", state);
state = await action("palette", { query: "keybindings" });
assert(state.paletteOpen && state.paletteResults.some((id) => id.endsWith(".11")), "palette finds nested activity", state);
state = await action("workspace", { id: "triple" });
assert(state.activeWorkspace === "triple", "Triple workspace switches", state);
state = await action("workspace", { id: "stacked" });
assert(state.activeWorkspace === "stacked", "Stacked workspace switches", state);
state = await action("workspace", { id: "default" });
state = await action("split", { pane: "1", direction: "horizontal" });
state = await action("drop", { activity: "5", destination: "2", edge: "bottom" });
assert(JSON.stringify(state.tree).includes("split-1") && JSON.stringify(state.tree).includes("drop-1"), "deterministic factories run", state);
state = await action("reset");
assert(state.splitSequence === 0 && state.dropSequence === 0 && !state.paletteOpen, "reset clears all bridge state", state);
assert(!JSON.stringify(state.tree).includes("split-1") && !JSON.stringify(state.tree).includes("drop-1"), "reset restores trees", state);
console.log(JSON.stringify({ ok: true, state }));
socket.close();
