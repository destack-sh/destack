import { expect, onTestFinished, test } from "@destack/test";
import { render } from "@solidjs/web";
import { type Accessor, flush } from "../solid/reactive.ts";
import {
    AppearanceScript,
    type ColorScheme,
    createAppearance,
    createColorScheme,
    type LocalAppearance,
} from "./appearance.tsx";

/** Run a component in a mounted root, removing it after the test. */
async function mount(component: () => null): Promise<void> {
    const host = document.createElement("main");
    document.body.append(host);
    const dispose = render(component, host);
    onTestFinished(() => {
        dispose();
        host.remove();
        localStorage.clear();
        document.documentElement.removeAttribute("style");
    });
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

test("restore the stored appearance from a script naming its storage key", () => {
    const host = document.createElement("div");
    const dispose = render(() => <AppearanceScript storageKey="site-appearance" />, host);

    // read the stored scheme under the key, else the default, and set it on the root
    expect(host.querySelector("script")?.innerHTML).toBe(
        'try{const a=localStorage.getItem("site-appearance")??"system";if(a==="light"||a==="dark")document.documentElement.style.colorScheme=a}catch{}',
    );
    dispose();
});

test("store a chosen appearance and set it on the root, the system's leaving the scheme to the device", async () => {
    let local: LocalAppearance | undefined;
    await mount(() => {
        local = createAppearance({ storageKey: "site-appearance" });

        return null;
    });

    // choose dark, then return to the system's appearance
    local?.setAppearance("dark");
    flush();
    const dark = [
        local?.appearance(),
        local?.resolvedAppearance(),
        localStorage.getItem("site-appearance"),
        document.documentElement.style.colorScheme,
    ];
    local?.setAppearance("system");
    flush();
    const system = [
        local?.appearance(),
        localStorage.getItem("site-appearance"),
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
