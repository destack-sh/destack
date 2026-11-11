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

test("unwrap optional, nullable and defaulted schemas to the schema they wrap", () => {
    const text = schema.string();

    expect(
        [
            text,
            text.exactOptional(),
            text.nullable(),
            text.default("none"),
            text.nullable().exactOptional(),
        ].map((type) => schema.unwrap(type) === text),
    ).toEqual([true, true, true, true, true]);
});
