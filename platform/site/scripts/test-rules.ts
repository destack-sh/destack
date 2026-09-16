import assert from "node:assert/strict";
import { enhanceRuleCatalog } from "../src/reader/rules.ts";

// exercise the generated directory's interaction contract without a browser dependency
const events = () => ({
    listeners: new Map<string, () => void>(),
    addEventListener(name: string, handler: () => void) { this.listeners.set(name, handler); },
    removeEventListener(name: string) { this.listeners.delete(name); },
});
const search = { name: "q", value: "", focus() { } };
const filters = ["category", "level", "fixability", "scope"].map((name) => ({ name, value: "", dataset: {} }));
const rows = [
    { hidden: false, dataset: { search: "danger security upstream-check", category: "security", level: "error", fixability: "automatic", scope: "module" } },
    { hidden: false, dataset: { search: "format style upstream-format", category: "style", level: "warning", fixability: "none", scope: "module" } },
];
const count = { textContent: "" }, empty = { hidden: false }, reset = events();
const controls = { ...events(), querySelector: () => search, querySelectorAll: (selector: string) => selector === "select" ? filters : [] };
const elements = { ".lint-controls": controls, ".lint-toolbar": {}, ".lint-count": count, ".lint-empty": empty, "[data-clear]": reset };
const catalog = { querySelector: (selector: string) => elements[selector as keyof typeof elements], querySelectorAll: () => rows };
let location = new URL("https://example.test/rules/?category=security&keep=yes#rules");
Object.defineProperty(globalThis, "location", { configurable: true, get: () => location });
Object.defineProperty(globalThis, "history", { configurable: true, value: { state: {}, replaceState(_state: unknown, _title: string, url: string | URL) { location = new URL(url); } } });
const window = events();
Object.defineProperty(globalThis, "window", { configurable: true, value: window });
const cleanup = enhanceRuleCatalog({ querySelector: () => catalog } as unknown as HTMLElement);
assert.equal(count.textContent, "1 of 2 rules");
assert.equal(rows[0].hidden, false);
assert.equal(rows[1].hidden, true);
search.value = "upstream-check";
controls.listeners.get("input")!();
assert.equal(count.textContent, "1 of 2 rules");
assert.equal(location.searchParams.get("keep"), "yes");
assert.equal(location.hash, "#rules");
search.value = "no-match";
controls.listeners.get("input")!();
assert.equal(count.textContent, "0 of 2 rules");
assert.equal(empty.hidden, false);
reset.listeners.get("click")!();
assert.equal(count.textContent, "2 rules");
assert.equal(location.searchParams.has("q"), false);
assert.equal(location.searchParams.has("category"), false);
cleanup();
assert.equal(controls.listeners.size, 0);
assert.equal(window.listeners.size, 0);
console.log("rule directory filtering passed");
