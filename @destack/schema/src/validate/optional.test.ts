import { expect, test } from "@destack/test";
import * as schema from "./index.ts";

test("decide which keys an object may leave out: exact optional and defaulted ones", () => {
    expect(
        [
            schema.string(),
            schema.string().exactOptional(),
            schema.string().default("none"),
            schema.string().nullable(),
        ].map((type) => schema.isOptional(type)),
    ).toEqual([false, true, true, false]);
});
