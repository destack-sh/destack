import { isServer } from "@solidjs/web";
import { type Accessor, action, createOptimistic, createSignal } from "solid-js";

/** A Web Share primitive's share action and its state. */
export type WebShareResult = {
    /** Share data through the device's share sheet. */
    readonly share: (data: ShareData) => Promise<void>;
    /** Whether a share is in progress. */
    readonly pending: Accessor<boolean>;
    /** Whether the last share succeeded, undefined before the first. */
    readonly status: Accessor<boolean | undefined>;
    /** Why the last share failed, undefined unless it did. */
    readonly message: Accessor<string | undefined>;
};

/** Make a share function through the Web Share API, rejecting where the browser cannot share the data. */
export function makeWebShare(): (data: ShareData) => Promise<void> {
    return async (data) => {
        // refuse where the browser has no share sheet, or cannot share these files
        if (!("share" in navigator)) {
            throw new TypeError("this browser cannot share");
        }
        if (data.files !== undefined && !navigator.canShare(data)) {
            throw new TypeError("this browser cannot share these files");
        }

        await navigator.share(data);
    };
}

/** Share through the Web Share API as an action, following whether it is pending, succeeded, and why it failed. */
export function createWebShare(): WebShareResult {
    // share nothing on the server
    if (isServer) {
        return {
            share: async () => {},
            pending: () => false,
            status: () => undefined,
            message: () => undefined,
        };
    }

    // follow the share's progress and outcome
    const [pending, setPending] = createOptimistic(false, { ownedWrite: true });
    const [status, setStatus] = createSignal<boolean | undefined>(undefined, { ownedWrite: true });
    const [message, setMessage] = createSignal<string | undefined>(undefined, { ownedWrite: true });
    const share = makeWebShare();

    // share as an action, recording success or the failure's message
    const run = action(function* (data: ShareData) {
        // start pending, clearing the last outcome
        setPending(true);
        setStatus(undefined);
        setMessage(undefined);
        try {
            yield share(data);
            setStatus(true);
        } catch (error) {
            setStatus(false);
            setMessage(error instanceof Error ? error.message : String(error));
        }
        setPending(false);
    });

    return { share: run, pending, status, message };
}
