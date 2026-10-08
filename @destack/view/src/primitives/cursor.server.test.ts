import { expect, test } from "@destack/test";
import { createBodyCursor, createElementCursor } from "./cursor.ts";

test("read no target or cursor on the server", () => {
    const read: string[] = [];
    createBodyCursor(() => {
        read.push("body");

        return false;
    });
    createElementCursor(
        () => {
            read.push("element");

            return false;
        },
        () => {
            read.push("cursor");

            return "pointer";
        },
    );

    expect(read).toEqual([]);
});
