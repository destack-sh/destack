import { expect, test } from "@destack/test";
import { defined } from "./defined.ts";

test("leave out undefined entries at every depth, keeping nulls, lists and bytes", () => {
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
