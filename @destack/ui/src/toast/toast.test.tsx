import { present } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { flush } from "@destack/view";
import { Toaster, type ToastManager, useToast } from "./index.ts";
import { render, stubPopovers, testWorld, wait } from "@destack/view/test";

/** The milliseconds past a 30 ms toast's end, which the simulated clock sleeps. */
const PAST_DURATION = 60;

/** The milliseconds past a dismissed toast's slide out, Sonner's 200 and some. */
const PAST_LEAVING = 250;

/** The milliseconds a toast with an action shows. */
const ACTION_TIME = 8000;

/** Render a toaster, answering its element and its toasts. */
function drawToaster(duration: number): {
    readonly container: HTMLElement;
    readonly toast: ToastManager;
} {
    // read the toasts from inside the toaster
    stubPopovers();
    let toast: ToastManager | undefined;
    const Page = () => {
        toast = useToast();

        return null;
    };
    const { container } = render(() => (
        <Toaster duration={duration}>
            <Page />
        </Toaster>
    ));

    return { container, toast: present(toast, "the toaster's toasts") };
}

/** Make a pointer event at a position along the swipe's axis. */
function pointer(type: string, position: number): PointerEvent {
    return new PointerEvent(type, { bubbles: true, clientX: position, pointerId: 1 });
}

/** Press a key on an element. */
function press(target: Element, key: string): void {
    target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

/** Read each shown toast as its kind and title, newest first. */
function shown(container: Element): string[] {
    return [
        ...container.querySelectorAll("[data-slot=toast]:not([data-leaving], [data-limited])"),
    ].map(
        (entry) =>
            `${entry.getAttribute("data-type")}: ${entry.querySelector("[data-slot=toast-title]")?.textContent}`,
    );
}

test("announce toasts in a labelled polite live region, newest first", () => {
    const { container, toast } = drawToaster(1000);
    toast.show("Note saved");
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
    const { container, toast } = drawToaster(30);
    toast.show("Archived", { onDismiss: () => dismissed.push("Archived") });
    toast.error("Copying failed");
    flush();
    await testWorld().simulation.sleep(PAST_DURATION);
    flush();
    const afterDuration = shown(container);
    container.querySelector<HTMLElement>("[data-slot=toast-close]")?.click();
    flush();
    expect([afterDuration, shown(container), dismissed]).toEqual([
        ["error: Copying failed"],
        [],
        ["Archived"],
    ]);
});

test("show a toast while the focus rests on it after the pointer left, dismissing it once both left", async () => {
    const { container, toast } = drawToaster(1000);
    toast.show("Archived", { duration: 30, action: { label: "Undo", onClick: () => {} } });
    flush();
    const stack = container.querySelector("[data-slot=toaster-stack]");
    const action = container.querySelector<HTMLElement>("[data-slot=toast-action]");
    stack?.dispatchEvent(new PointerEvent("pointerenter"));
    action?.focus();
    stack?.dispatchEvent(new PointerEvent("pointerleave"));
    await testWorld().simulation.sleep(PAST_DURATION);
    flush();
    const shownFirst = shown(container);
    action?.blur();
    await testWorld().simulation.sleep(PAST_DURATION);
    flush();
    expect([shownFirst, shown(container)]).toEqual([["default: Archived"], []]);
});

test("run a toast's action and dismiss it, and replace a toast by its id", () => {
    const undone: string[] = [];
    const { container, toast } = drawToaster(1000);
    const id = toast.show("Moved to Trips", {
        action: { label: "Undo", onClick: () => undone.push("undo") },
    });
    toast.info("Moved to Work", { id });
    flush();
    const replaced = shown(container);
    toast.show("Deleted", { action: { label: "Undo", onClick: () => undone.push("delete") } });
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
    const { container, toast } = drawToaster(1000);
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
    const { container, toast } = drawToaster(1000);
    for (const title of ["one", "two", "three", "four"]) {
        toast.show(title);
    }
    flush();
    expect(shown(container)).toEqual(["default: four", "default: three", "default: two"]);
});

test("leave the live region in the document and show the stack in the top layer only while toasts exist", async () => {
    const { container, toast } = drawToaster(1000);
    const region = container.querySelector("section");
    const stack = container.querySelector("[data-slot=toaster-stack]");
    const empty = [region?.getAttribute("aria-live"), stack?.hasAttribute("data-popover-open")];
    const id = toast.show("Note saved");
    flush();
    const isShown = [
        stack?.getAttribute("popover"),
        stack?.hasAttribute("data-popover-open"),
        stack?.parentElement === region,
    ];
    toast.dismiss(id);
    flush();
    const leaving = stack?.hasAttribute("data-popover-open");

    // the stack stays while the toast slides out, and leaves the top layer once it unmounted
    await testWorld().simulation.sleep(PAST_LEAVING);
    flush();
    expect([
        empty,
        isShown,
        leaving,
        region?.isConnected,
        stack?.hasAttribute("data-popover-open"),
    ]).toEqual([["polite", false], ["manual", true, true], true, true, false]);
});

test("move the focus to the toasts and back with F6, opening the stack, and off a toast Escape dismisses", () => {
    // start from a focused button on the page
    const { container, toast } = drawToaster(1000);
    const button = document.createElement("button");
    document.body.append(button);
    onTestFinished(() => button.remove());
    const region = container.querySelector<HTMLElement>("section");
    const stack = container.querySelector("[data-slot=toaster-stack]");
    const focused = () =>
        document.activeElement === button
            ? "page"
            : document.activeElement === region
              ? "toasts"
              : document.activeElement?.getAttribute("data-slot");
    button.focus();

    // leave F6 to the page without toasts, then go to them and back
    const steps: (readonly [string | null | undefined, boolean | undefined])[] = [];
    const record = () => {
        flush();
        steps.push([focused(), stack?.hasAttribute("data-expanded")]);
    };
    press(document.body, "F6");
    record();
    toast.error("Upload failed");
    toast.error("Saving failed");
    flush();
    press(document.body, "F6");
    record();
    press(present(region, "the region"), "F6");
    record();

    // dismiss both from their close buttons with Escape, going back to the page after the last
    press(document.body, "F6");
    for (let count = 0; count < 2; count += 1) {
        const close = container.querySelector<HTMLElement>(
            "[data-slot=toast]:not([data-leaving], [data-limited]) [data-slot=toast-close]",
        );
        close?.focus();
        press(present(close, "a close button"), "Escape");
        record();
    }

    expect([steps, shown(container)]).toEqual([
        [
            ["page", false],
            ["toasts", true],
            ["page", false],
            ["toasts", true],
            ["page", false],
        ],
        [],
    ]);
});

test("show an error until dismissed, an action for longer, and others for the toaster's time", async () => {
    const { container, toast } = drawToaster(30);
    toast.error("Upload failed");
    toast.loading("Uploading");
    toast.show("Archived", { action: { label: "Undo", onClick: () => {} } });
    flush();
    await testWorld().simulation.sleep(PAST_DURATION);
    flush();
    const early = shown(container);
    await testWorld().simulation.sleep(ACTION_TIME);
    flush();
    expect([early, shown(container)]).toEqual([
        ["default: Archived", "loading: Uploading", "error: Upload failed"],
        ["loading: Uploading", "error: Upload failed"],
    ]);
});

test("dismiss a toast swiped off the toaster's side, leaving one swiped a little", () => {
    const { container, toast } = drawToaster(10_000);
    toast.show("Note saved", { id: "saved" });
    flush();
    const swipeRight = (distance: number): void => {
        const entry = container.querySelector("[data-slot=toast-title]");
        entry?.dispatchEvent(pointer("pointerdown", 0));
        entry?.dispatchEvent(pointer("pointermove", distance));
        flush();
        entry?.dispatchEvent(pointer("pointerup", distance));
        flush();
    };
    swipeRight(4);
    const remaining = shown(container);
    swipeRight(300);
    expect([remaining, shown(container)]).toEqual([["default: Note saved"], []]);
});

test("keep a toast pushed out by newer ones to fade, and bring it back as they leave", () => {
    const { container, toast } = drawToaster(1000);
    const ids = Array.from({ length: 9 }, (_, index) => toast.show(`Note ${String(index)}`));
    flush();
    const read = (state: string): string[] =>
        [...container.querySelectorAll(`[data-slot=toast]${state}`)].map(
            (entry) => entry.querySelector("[data-slot=toast-title]")?.textContent ?? "no title",
        );
    const limited = read("[data-limited][inert]");
    const inView = shown(container);
    toast.dismiss(ids.at(-1));
    flush();

    expect({ inView, limited, after: shown(container), left: read("[data-limited]") }).toEqual({
        inView: ["default: Note 8", "default: Note 7", "default: Note 6"],
        limited: ["Note 5", "Note 4", "Note 3"],
        after: ["default: Note 7", "default: Note 6", "default: Note 5"],
        left: ["Note 4", "Note 3", "Note 2"],
    });
});
