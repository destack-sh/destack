import { expect, onTestFinished, test, vi } from "@destack/test";
import { type Component, createComponent } from "solid-js";
import { render } from "../test/dom.ts";
import { renderOnServer } from "../test/hydration.ts";
import * as fixture from "./primitives.fixture.tsx";

// stand in for the browser features the test DOM lacks
Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: Object.assign(new EventTarget(), { read: async () => [], readText: async () => "" }),
});
Object.defineProperty(navigator, "mediaDevices", {
    configurable: true,
    value: Object.assign(new EventTarget(), { enumerateDevices: async () => [] }),
});
Object.defineProperty(navigator, "permissions", {
    configurable: true,
    value: { query: async () => Object.assign(new EventTarget(), { state: "prompt" }) },
});
Object.defineProperty(screen, "orientation", {
    configurable: true,
    value: Object.assign(new EventTarget(), { angle: 0, type: "portrait-primary" }),
});

/** The server's HTML of every fixture component, by export name. */
const rendered = await renderOnServer("src/primitives/primitives.fixture.tsx");

/** The fixture's components by export name, past the exports the compiler adds. */
const components = Object.entries(fixture).filter(([name]) => !name.startsWith("$$"));

test.each(components)(
    "hydrate %s without shifting the server's hydration keys",
    (name, component: Component) => {
        // collect the warnings hydration gives for keys and nodes it misses
        const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
        onTestFinished(() => warn.mockRestore());

        // hydrate the server's paragraph, which claims it only when the keys stay aligned
        const html = rendered[name] ?? "";
        const container = document.createElement("div");
        container.innerHTML = html;
        render(() => createComponent(component, {}), { container, hydrate: true });
        const paragraph = container.querySelector("p");

        expect([
            warn.mock.calls.map((call) => String(call[0])),
            paragraph?.hasAttribute("_hk") === true && container.children.length === 1,
        ]).toEqual([[], true]);
    },
);
