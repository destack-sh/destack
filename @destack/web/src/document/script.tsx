import type { JSX } from "@solidjs/web";
import { isServer, onSettled } from "@destack/view";
import * as head from "../head/index.ts";

/** When a third-party script loads, after next/script's strategies. */
export type ScriptStrategy = "beforeInteractive" | "afterInteractive" | "lazyOnload";

/** The properties of a third-party script. */
export interface ScriptProperties {
    /** The script's address. */
    readonly src: string;
    /** When it loads: in the head before the page hydrates, once it has, or when the browser is idle after load. */
    readonly strategy?: ScriptStrategy;
    /** Run once the script loaded. */
    readonly onLoad?: () => void;
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
        onSettled(() => {
            if (strategy === "afterInteractive") {
                add(properties);
            } else {
                whenIdle(() => add(properties));
            }
        });
    }

    return null;
}

/** Add a script to the body once per address. */
function add(properties: ScriptProperties): void {
    // skip a script the page holds already
    const added = [...document.scripts].some(
        (script) => script.getAttribute("src") === properties.src,
    );
    if (added) {
        return;
    }

    // append it, reporting its load
    const element = document.createElement("script");
    element.src = properties.src;
    element.async = true;
    if (properties.onLoad !== undefined) {
        element.addEventListener("load", properties.onLoad, { once: true });
    }
    document.body.append(element);
}

/** Run once the page loaded and the browser is idle. */
function whenIdle(run: () => void): void {
    // wait for the load event when the page still loads
    const idle = () =>
        "requestIdleCallback" in window ? requestIdleCallback(run) : setTimeout(run, 1);
    if (document.readyState === "complete") {
        idle();
    } else {
        window.addEventListener("load", idle, { once: true });
    }
}
