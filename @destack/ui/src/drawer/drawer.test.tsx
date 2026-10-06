import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { draw, markup } from "@destack/view/test";
import { Drawer, DrawerContent, DrawerTitle } from "./index.ts";

test("place a drawer against its direction, with a handle above a bottom drawer's content", () => {
    const container = draw(() => (
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
        '<dialog id="id-1" data-slot="drawer-content" closedby="any" aria-labelledby="id-2" data-side="bottom">' +
            '<div data-slot="drawer-handle" aria-hidden="true"></div>' +
            '<h2 id="id-2" data-slot="drawer-title">Share</h2></dialog>' +
            '<dialog id="id-3" data-slot="drawer-content" closedby="any" aria-labelledby="id-4" data-side="right">' +
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
    const container = draw(() => (
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
