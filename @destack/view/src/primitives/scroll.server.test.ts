import { expect, test } from "@destack/test";
import {
    createPreventScroll,
    createScrollPosition,
    getScrollPosition,
    useWindowScrollPosition,
} from "./scroll.ts";

test("read zero scroll and prevent nothing on the server", () => {
    createPreventScroll({ enabled: true, hideScrollbar: true });

    expect([
        getScrollPosition(undefined),
        { ...createScrollPosition() },
        { ...useWindowScrollPosition() },
    ]).toEqual([
        { x: 0, y: 0 },
        { x: 0, y: 0 },
        { x: 0, y: 0 },
    ]);
});
