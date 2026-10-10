import { isServer } from "@solidjs/web";
import { type Accessor, onCleanup } from "solid-js";
import { createHydratableSingletonRoot } from "./rootless.ts";
import { createHydratableSignal } from "./utils.ts";

/** The browsers' default root font size in pixels, which the server renders with, knowing no client's own. */
const DEFAULT_REM_SIZE = 16;

/** Read the root's font size, the rem, in pixels. */
export function getRemSize(): number {
    return isServer
        ? DEFAULT_REM_SIZE
        : Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
}

/** Follow the rem size in pixels as the root's font size changes. */
export function createRemSize(): Accessor<number> {
    // answer the server's size on the server
    if (isServer) {
        return () => DEFAULT_REM_SIZE;
    }
    const [remSize, setRemSize] = createHydratableSignal(DEFAULT_REM_SIZE, getRemSize);

    // measure a hidden element one rem wide, rereading the rem whenever it resizes after its first report
    const probe = document.createElement("div");
    probe.setAttribute("aria-hidden", "true");
    probe.style.cssText =
        "border: 0; padding: 0; visibility: hidden; position: absolute; top: -9999px; left: -9999px; width: 1rem;";
    document.body.append(probe);
    let isFirst = true;
    const observer = new ResizeObserver(() => {
        if (isFirst) {
            isFirst = false;
        } else {
            setRemSize(getRemSize());
        }
    });
    observer.observe(probe);
    onCleanup(() => {
        observer.disconnect();
        probe.remove();
    });

    return remSize;
}

/** Follow the rem size through one measurement shared by every user. */
export const useRemSize: () => Accessor<number> = createHydratableSingletonRoot(createRemSize);
