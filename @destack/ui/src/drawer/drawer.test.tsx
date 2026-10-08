import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { markup, render, wait } from "@destack/view/test";
import { Drawer, DrawerContent, DrawerTitle } from "./index.ts";

test("place a drawer against its direction, with a handle above a bottom drawer's content", () => {
    const { container } = render(() => (
        <>
            <Drawer>
                <DrawerContent>
                    <DrawerTitle>Share</DrawerTitle>
                </DrawerContent>
            </Drawer>
            <Drawer direction="right">
                <DrawerContent>
                    <DrawerTitle>Details</DrawerTitle>
                </DrawerContent>
            </Drawer>
        </>
    ));
    expect(markup(container)).toBe(
        '<dialog id="id-1" data-slot="drawer-content" data-state="closed" closedby="any" aria-labelledby="id-2" data-side="bottom">' +
            '<div data-slot="drawer-handle" aria-hidden="true"></div>' +
            '<h2 id="id-2" data-slot="drawer-title">Share</h2></dialog>' +
            '<dialog id="id-3" data-slot="drawer-content" data-state="closed" closedby="any" aria-labelledby="id-4" data-side="right">' +
            '<h2 id="id-4" data-slot="drawer-title">Details</h2></dialog>',
    );
});

/** Make a pointer event at a position along the swipe's axis. */
function pointer(type: string, position: number): PointerEvent {
    return new PointerEvent(type, { bubbles: true, clientY: position, pointerId: 1 });
}

/** Drag a drawer by a pointer from one point down to another. */
function dragDown(drawer: Element, from: number, to: number): void {
    drawer.dispatchEvent(pointer("pointerdown", from));
    drawer.dispatchEvent(pointer("pointermove", to));
    flush();
    drawer.dispatchEvent(pointer("pointerup", to));
    flush();
}

test("close a drawer dragged toward its edge past a share of it, and spring back from a short drag", () => {
    const changes: boolean[] = [];
    const { container } = render(() => (
        <Drawer defaultOpen onOpenChange={(open) => changes.push(open)}>
            <DrawerContent>
                <DrawerTitle>Share</DrawerTitle>
            </DrawerContent>
        </Drawer>
    ));
    const drawer = container.querySelector("[data-slot=drawer-content]");
    if (drawer === null) {
        throw new TypeError("the drawer did not render");
    }
    dragDown(drawer, 100, 102);
    const short = [...changes];
    dragDown(drawer, 100, 400);
    expect([short, changes]).toEqual([[], [false]]);
});

/** Drag a drawer by a pointer from one point to another and hold it there before letting go. */
async function dragAndHold(drawer: Element, from: number, to: number): Promise<void> {
    drawer.dispatchEvent(pointer("pointerdown", from));
    drawer.dispatchEvent(pointer("pointermove", to));
    flush();
    await wait(20);
    drawer.dispatchEvent(pointer("pointermove", to));
    drawer.dispatchEvent(pointer("pointerup", to));
    flush();
}

test("settle a dragged drawer at the nearest snap point, carry a flick on, and close it dragged past the smallest", async () => {
    const points: (number | string)[] = [];
    const changes: boolean[] = [];
    const { container } = render(() => (
        <Drawer
            defaultOpen
            snapPoints={[0.5, 1]}
            onSnapPointChange={(point) => points.push(point)}
            onOpenChange={(open) => changes.push(open)}
        >
            <DrawerContent>
                <DrawerTitle>Share</DrawerTitle>
            </DrawerContent>
        </Drawer>
    ));
    const drawer = container.querySelector("[data-slot=drawer-content]");
    if (drawer === null) {
        throw new TypeError("the drawer did not render");
    }
    const rested = drawer.getAttribute("data-snap-point");

    // drop 300 of the viewport's 768 pixels and hold, nearer half the viewport than all of it
    await dragAndHold(drawer, 100, 400);
    const held = drawer.getAttribute("data-snap-point");

    // flick up 50 pixels, which the speed carries past the largest
    dragDown(drawer, 400, 350);
    const flicked = drawer.getAttribute("data-snap-point");

    // settle at half the viewport again, then drop to 84 pixels, past a quarter below the smallest
    await dragAndHold(drawer, 100, 400);
    await dragAndHold(drawer, 100, 400);
    expect([rested, held, flicked, points, changes]).toEqual([
        "1",
        "0",
        "1",
        [0.5, 1, 0.5],
        [false],
    ]);
});
