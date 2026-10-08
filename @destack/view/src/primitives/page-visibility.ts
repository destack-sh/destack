import { isServer } from "@solidjs/web";
import type { Accessor } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { createHydratableSingletonRoot } from "./rootless.ts";
import { createHydratableSignal } from "./utils.ts";

/** Follow whether the page is visible, true on the server. */
export function createPageVisibility(): Accessor<boolean> {
    // read the page as visible on the server
    if (isServer) {
        return () => true;
    }
    const [visibility, setVisibility] = createHydratableSignal(true, isVisible);
    makeEventListener(document, "visibilitychange", () => setVisibility(isVisible()));

    return visibility;
}

/** Follow whether the page is visible through one listener shared by every user. */
export const usePageVisibility: () => Accessor<boolean> =
    createHydratableSingletonRoot(createPageVisibility);

/** Read whether the page is visible. */
function isVisible(): boolean {
    return document.visibilityState === "visible";
}
