import { expect, test } from "@destack/test";
import { defined } from "./defined.ts";

test("leave out undefined entries at every depth, leaving nulls, lists and bytes as they are", () => {
    const bytes = new Uint8Array([1, 2]);
    expect(
        defined({
            title: "Plans",
            note: undefined,
            due: null,
            owner: { name: "Ada", handle: undefined },
            tags: [{ label: "red", color: undefined }],
            bytes,
        }),
    ).toEqual({
        title: "Plans",
        due: null,
        owner: { name: "Ada" },
        tags: [{ label: "red" }],
        bytes,
    });
});
