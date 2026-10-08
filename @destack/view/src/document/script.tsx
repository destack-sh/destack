import type { JSX } from "@solidjs/web";
import { isServer } from "../solid/component.ts";
import { onMount } from "../primitives/utils.ts";
import * as head from "./head.ts";

/** When a third-party script loads. */
export type ScriptStrategy = "beforeInteractive" | "afterInteractive" | "lazyOnload";

/** The properties of a third-party script. */
export interface ScriptProperties {
    /** The script's address. */
    readonly src: string;
    /** When it loads: in the head before the page hydrates, once it has, or when the browser is idle after load. */
    readonly strategy?: ScriptStrategy;
    /** Call back once the script loads. */
    readonly onLoad?: () => void;
    /** Call back once the script loads, and on every later mount while it stays loaded. */
    readonly onReady?: () => void;
    /** Call back when the script fails to load. */
    readonly onError?: (event: Event) => void;
}

/** Load a third-party script once, in the head or after the page hydrates. */
export function Script(properties: ScriptProperties): JSX.Element {
    // write a script that loads before hydration into the head
    const strategy = properties.strategy ?? "afterInteractive";
    if (strategy === "beforeInteractive") {
        return <head.Script src={properties.src} />;
    }

    // add the others on the client once the page settles, or once it is idle after load
    if (!isServer) {
        onMount(() => {
            if (strategy === "afterInteractive") {
                add(properties);
            } else {
                whenIdle(() => add(properties));
            }
        });
    }

    return null;
}

/** The addresses of the scripts that finished loading. */
const loaded = new Set<string>();

/** Add a script to the body once per address, telling a later mount it is ready once loaded. */
function add(properties: ScriptProperties): void {
    // tell a later mount the script is ready, and skip a script the page holds already
    const added = [...document.scripts].some(
        (script) => script.getAttribute("src") === properties.src,
    );
    if (added) {
        if (loaded.has(properties.src)) {
            properties.onReady?.();
        }

        return;
    }

    // append it, reporting its load or failure
    const element = document.createElement("script");
    element.src = properties.src;
    element.async = true;
    element.addEventListener(
        "load",
        () => {
            loaded.add(properties.src);
            properties.onLoad?.();
            properties.onReady?.();
        },
        { once: true },
    );
    if (properties.onError !== undefined) {
        element.addEventListener("error", properties.onError, { once: true });
    }
    document.body.append(element);
}

/** Run once the page loaded and the browser is idle. */
function whenIdle(run: () => void): void {
    // wait for the load event when the page still loads
    const idle = () =>
        "requestIdleCallback" in window ? requestIdleCallback(run) : setTimeout(run);
    if (document.readyState === "complete") {
        idle();
    } else {
        window.addEventListener("load", idle, { once: true });
    }
}
