import { afterEach, expect, test } from "@destack/test";
import { createRoot, flush, runWithOwner } from "solid-js";
import { createWebShare, makeWebShare } from "./share.ts";

afterEach(() => {
    Reflect.deleteProperty(navigator, "share");
    Reflect.deleteProperty(navigator, "canShare");
});

/** Give the browser a share sheet that resolves or rejects. */
function stubShare(outcome: "resolve" | Error): ShareData[] {
    const shared: ShareData[] = [];
    Object.defineProperty(navigator, "share", {
        configurable: true,
        value: async (data: ShareData) => {
            shared.push(data);
            if (outcome !== "resolve") {
                throw outcome;
            }
        },
    });
    Object.defineProperty(navigator, "canShare", { configurable: true, value: () => false });

    return shared;
}

test("refuse to share without a share sheet, and files the browser cannot share", async () => {
    // share without a share sheet, then files with one
    const share = makeWebShare();
    const without = await share({ url: "https://destack.sh" }).catch((error: unknown) => error);
    stubShare("resolve");
    const files = await share({ files: [new File(["x"], "x.txt")] }).catch(
        (error: unknown) => error,
    );

    expect([without, files]).toEqual([
        new TypeError("this browser cannot share"),
        new TypeError("this browser cannot share these files"),
    ]);
});

test("follow a share from pending to success, and to failure with its reason", async () => {
    const observed = await createRoot(async (disposeRoot) => {
        // share successfully
        const shared = stubShare("resolve");
        const { share, pending, status, message } = createWebShare();
        const initial = [pending(), status(), message()];
        const promise = runWithOwner(null, () => share({ url: "https://destack.sh" }));
        flush();
        const during = pending();
        await promise;
        flush();
        const succeeded = [pending(), status(), message(), shared.length];

        // share and fail
        stubShare(new Error("share dismissed"));
        await runWithOwner(null, () => share({ url: "https://destack.sh" }));
        flush();
        const failed = [pending(), status(), message()];
        disposeRoot();

        return { initial, during, succeeded, failed };
    });

    expect(observed).toEqual({
        initial: [false, undefined, undefined],
        during: true,
        succeeded: [false, true, undefined, 1],
        failed: [false, false, "share dismissed"],
    });
});
