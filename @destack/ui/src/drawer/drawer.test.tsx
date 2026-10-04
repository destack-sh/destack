import { expect, test } from "@destack/test";
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
