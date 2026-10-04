import { expect, onTestFinished, test } from "@destack/test";
import { flush } from "solid-js";
import { toast, Toaster } from "./index.ts";
import { draw, stubPopovers, wait } from "@destack/view/test";

/** Render a toaster that clears the page's toasts after the test. */
function drawToaster(duration: number): HTMLElement {
    stubPopovers();
    onTestFinished(() => toast.dismiss());

    return draw(() => <Toaster duration={duration} />);
}

/** Read each shown toast as its kind and title, newest first. */
function shown(container: Element): string[] {
    return [...container.querySelectorAll("[data-slot=toast]")].map(
        (entry) =>
            `${entry.getAttribute("data-type")}: ${entry.querySelector("[data-slot=toast-title]")?.textContent}`,
    );
}

test("announce toasts in a labelled polite live region, newest first", () => {
    const container = drawToaster(1000);
    toast("Note saved");
    toast.error("Upload failed", { description: "The file is too large." });
    flush();
    const region = container.querySelector("section");
    expect([
        region?.getAttribute("aria-label"),
        region?.getAttribute("aria-live"),
        shown(container),
    ]).toEqual(["Notifications", "polite", ["error: Upload failed", "default: Note saved"]]);
    expect(container.querySelector("[data-slot=toast-description]")?.textContent).toBe(
        "The file is too large.",
    );
});

test("dismiss a toast after its duration, or at once from its close button", async () => {
    const dismissed: string[] = [];
    const container = drawToaster(30);
    toast("Archived", { onDismiss: () => dismissed.push("Archived") });
    toast.success("Copied", { duration: Number.POSITIVE_INFINITY });
    flush();
    await wait(60);
    flush();
    const afterDuration = shown(container);
    container.querySelector<HTMLElement>("[data-slot=toast-close]")?.click();
    flush();
    expect([afterDuration, shown(container), dismissed]).toEqual([
        ["success: Copied"],
        [],
        ["Archived"],
    ]);
});

test("run a toast's action and dismiss it, and replace a toast by its id", () => {
    const undone: string[] = [];
    const container = drawToaster(1000);
    const id = toast("Moved to Trips", {
        action: { label: "Undo", onClick: () => undone.push("undo") },
    });
    toast.info("Moved to Work", { id });
    flush();
    const replaced = shown(container);
    toast("Deleted", { action: { label: "Undo", onClick: () => undone.push("delete") } });
    flush();
    container.querySelector<HTMLElement>("[data-slot=toast-action]")?.click();
    flush();
    expect([replaced, undone, shown(container)]).toEqual([
        ["info: Moved to Work"],
        ["delete"],
        ["info: Moved to Work"],
    ]);
});

test("follow a promise from a loading toast to its outcome", async () => {
    const container = drawToaster(1000);
    const saving = toast.promise(Promise.resolve(3), {
        loading: "Saving",
        success: (count) => `Saved ${count} notes`,
        error: "Saving failed",
    });
    flush();
    const pending = shown(container);
    await saving;
    await wait(0);
    flush();
    expect([pending, shown(container)]).toEqual([["loading: Saving"], ["success: Saved 3 notes"]]);
});

test("show the newest toasts up to the toaster's limit", () => {
    const container = drawToaster(1000);
    for (const title of ["one", "two", "three", "four"]) {
        toast(title);
    }
    flush();
    expect(shown(container)).toEqual(["default: four", "default: three", "default: two"]);
});

test("keep the live region in the document and show the stack in the top layer only while toasts exist", () => {
    const container = drawToaster(1000);
    const region = container.querySelector("section");
    const stack = container.querySelector("[data-slot=toaster-stack]");
    const empty = [region?.getAttribute("aria-live"), stack?.hasAttribute("data-popover-open")];
    const id = toast("Note saved");
    flush();
    const isShown = [
        stack?.getAttribute("popover"),
        stack?.hasAttribute("data-popover-open"),
        stack?.parentElement === region,
    ];
    toast.dismiss(id);
    flush();
    expect([empty, isShown, region?.isConnected, stack?.hasAttribute("data-popover-open")]).toEqual(
        [["polite", false], ["manual", true, true], true, false],
    );
});

test("move the focus to the toasts on Alt+T, whatever letter the layout types", () => {
    const container = drawToaster(1000);
    toast("Note saved");
    flush();
    document.body.dispatchEvent(
        new KeyboardEvent("keydown", { key: "†", code: "KeyT", altKey: true, bubbles: true }),
    );
    expect(document.activeElement).toBe(container.querySelector("[data-slot=toaster]"));
});
