import { isServer } from "@solidjs/web";
import { createEffect, onCleanup } from "solid-js";
import { type MaybeAccessor, TRANSPARENT } from "../primitives/utils.ts";

/** Ask the person to confirm before leaving the page, until the returned function is called. */
export function makePageLeave(): () => void {
    // block nothing on the server
    if (isServer) {
        return () => {};
    }
    window.addEventListener("beforeunload", block);

    return () => window.removeEventListener("beforeunload", block);
}

/** Ask the person to confirm before leaving the page while enabled, until cleanup. */
export function createPageLeaveBlocker(enabled: MaybeAccessor<boolean> = true): void {
    // block nothing on the server
    if (isServer) {
        return;
    }

    // block while a fixed flag says so, or whenever the accessor does
    if (typeof enabled === "function") {
        createEffect(
            enabled,
            (isEnabled) => (isEnabled ? makePageLeave() : undefined),
            TRANSPARENT,
        );
    } else if (enabled) {
        onCleanup(makePageLeave());
    }
}

/** Ask the person to confirm leaving. */
function block(event: BeforeUnloadEvent): void {
    event.preventDefault();
}
