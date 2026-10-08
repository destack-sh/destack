import { expect, test } from "@destack/test";
import { createElementSize, createResizeObserver, createWindowSize } from "./resize-observer.ts";

test("read no targets and keep fallback sizes on the server", () => {
    // observe through accessors that record being read
    const read: string[] = [];
    createResizeObserver(
        () => {
            read.push("targets");

            return undefined;
        },
        () => read.push("resize"),
    );
    const size = createElementSize((): false => {
        read.push("element");

        return false;
    });

    expect([read, { ...size }, { ...createWindowSize() }]).toEqual([
        [],
        { width: null, height: null, clientWidth: null, clientHeight: null },
        { width: 0, height: 0 },
    ]);
});
