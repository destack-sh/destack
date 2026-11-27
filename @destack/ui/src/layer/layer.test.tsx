import type { Scheduler } from "@destack/environment";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TopLayer } from "./index.ts";

test("follow the topmost modal as modals open and close in any order", () => {
    // open two modals, close the lower one first, then the top one
    const [lower, upper] = [document.createElement("dialog"), document.createElement("dialog")];
    const layer = new TopLayer();
    const tops: (string | undefined)[] = [];
    lower.id = "lower";
    upper.id = "upper";
    const stop = layer.subscribe((top) => tops.push(top?.id));
    layer.showModal(lower);
    layer.showModal(upper);
    layer.close(lower);
    layer.close(lower);
    layer.close(upper);
    stop();
    layer.showModal(lower);

    // closing a modal twice tells nobody the second time, and an unsubscribed handler hears nothing
    expect([tops, layer.top()?.id]).toEqual([["lower", "upper", "upper", undefined], "lower"]);
});

test("share one top layer per document", () => {
    const element = document.createElement("div");

    expect(TopLayer.of(element)).toBe(TopLayer.of(document.body));
});

/** A scheduler whose waiting tasks never run, so only a toggle ends a hold. */
const NEVER: Scheduler = {
    yield: () => new Promise<void>(() => {}),
    run: (start) => start(),
    waitUntil: (settled) => settled,
    task: (start) => start(),
};

test("run a show asked for inside another after it, and none an overlay stopped wanting", () => {
    // ask for a second and a third show while the first runs, the third no longer wanted
    const layer = new TopLayer();
    const [first, second, third] = ["first", "second", "third"].map(() =>
        document.createElement("div"),
    );
    const ran: string[] = [];
    layer.showPopover(
        present(first, "a popover"),
        () => {
            ran.push("first");
            layer.showPopover(
                present(second, "a popover"),
                () => ran.push("second"),
                () => true,
            );
            layer.showPopover(
                present(third, "a popover"),
                () => ran.push("third"),
                () => false,
            );
            ran.push("first done");
        },
        () => true,
    );

    expect(ran).toEqual(["first", "first done", "second"]);
});

test("wait with shows from a popover's beforetoggle to its toggle, or a refused show", () => {
    // watch a popover, start its show, and ask for another show meanwhile
    const layer = new TopLayer();
    const [popover, other] = [document.createElement("div"), document.createElement("div")];
    const ran: string[] = [];
    layer.watch(popover, NEVER);
    popover.dispatchEvent(new Event("beforetoggle"));
    layer.showPopover(
        other,
        () => ran.push("shown"),
        () => true,
    );
    const held = [...ran];

    // its toggle ends the wait, and the show runs once
    popover.dispatchEvent(new Event("toggle"));
    popover.dispatchEvent(new Event("toggle"));

    expect([held, ran]).toEqual([[], ["shown"]]);
});

test("end the wait at once where a popover's show is refused", async () => {
    // refuse the show in the popover's own handler
    const layer = new TopLayer();
    const [popover, other] = [document.createElement("div"), document.createElement("div")];
    const ran: string[] = [];
    layer.watch(popover, NEVER);
    popover.addEventListener("beforetoggle", (event) => event.preventDefault());
    popover.dispatchEvent(new Event("beforetoggle", { cancelable: true }));
    layer.showPopover(
        other,
        () => ran.push("shown"),
        () => true,
    );
    await Promise.resolve();

    expect(ran).toEqual(["shown"]);
});
