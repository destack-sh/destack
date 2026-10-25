import { testSimulation } from "@destack/test/world";
import { expect, onTestFinished, test } from "@destack/test";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import type { BrowserEnvironment } from "@destack/environment/browser";
import { render } from "../test/dom.ts";
import { type Accessor, flush } from "../solid/reactive.ts";
import {
    AppearanceScript,
    type ColorScheme,
    createAppearance,
    createColorScheme,
    type LocalAppearance,
} from "./appearance.tsx";

/** Run a component in a root of a tab of its own, answering the tab. */
async function mount(component: () => null): Promise<BrowserEnvironment> {
    // render in a fresh tab, and reset the root's scheme after the test
    const simulation = testSimulation();
    onTestFinished(() => simulation[Symbol.asyncDispose]());
    const tab = new BrowserSimulation(simulation).environment("tab");
    render(component, { environment: tab });
    onTestFinished(() => document.documentElement.removeAttribute("style"));
    flush();

    return tab;
}

test("restore the stored appearance from a script naming its storage key", () => {
    const host = document.createElement("div");
    const { unmount } = render(() => <AppearanceScript storageKey="site-appearance" />, {
        container: host,
    });

    // read the stored scheme under the key, else the default, and set it on the root
    expect(host.querySelector("script")?.innerHTML).toBe(
        'try{const a=localStorage.getItem("site-appearance")??"system";if(a==="light"||a==="dark")document.documentElement.style.colorScheme=a}catch{}',
    );
    unmount();
});

test("store a chosen appearance and set it on the root, the system's leaving the scheme to the device", async () => {
    let local: LocalAppearance | undefined;
    const tab = await mount(() => {
        local = createAppearance({ storageKey: "site-appearance" });

        return null;
    });

    // choose dark, then return to the system's appearance
    local?.setAppearance("dark");
    flush();
    const dark = [
        local?.appearance(),
        local?.resolvedAppearance(),
        tab.storage.getItem("site-appearance"),
        document.documentElement.style.colorScheme,
    ];
    local?.setAppearance("system");
    flush();
    const system = [
        local?.appearance(),
        tab.storage.getItem("site-appearance"),
        document.documentElement.style.colorScheme,
    ];

    // dark is stored and set, the system's is stored but leaves the scheme to the device
    expect({ dark, system }).toEqual({
        dark: ["dark", "dark", "dark", "dark"],
        system: ["system", "system", "light dark"],
    });
});

test("follow the scheme of a dark theme root", async () => {
    const panel = document.createElement("div");
    panel.style.colorScheme = "dark";
    document.body.append(panel);
    onTestFinished(() => panel.remove());
    let scheme: Accessor<ColorScheme> | undefined;
    await mount(() => {
        scheme = createColorScheme(() => panel);

        return null;
    });

    // resolve dark from the root's own scheme
    expect(scheme?.()).toBe("dark");
});
