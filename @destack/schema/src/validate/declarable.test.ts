import { z } from "zod";
import { expect, test } from "@destack/test";
import { requireDeclarable } from "./declarable.ts";

test("accept schemas of JSON values with exportable rules", () => {
    const declared = z.strictObject({
        name: z
            .string()
            .min(1)
            .max(63)
            .regex(/^[a-z]+$/),
        count: z.number().int().nonnegative(),
        tags: z.array(z.enum(["a", "b"])).optional(),
        parent: z.union([z.string(), z.null()]),
    });

    expect(() => requireDeclarable(declared)).not.toThrow();
});

test("refuse schemas that coerce, run code, allow unknown properties or miss values outside objects", () => {
    expect(
        [
            z.coerce.number(),
            z.string().refine((value) => value.length > 0),
            z.object({ name: z.string() }),
            z.array(z.string().optional()),
            z.date(),
            z.string().regex(/a/g),
            z.string().meta({ default: "x" }),
        ].map((declared) => {
            try {
                requireDeclarable(declared);

                return "accepted";
            } catch (error) {
                return (error as Error).message;
            }
        }),
    ).toEqual([
        "declared schemas cannot coerce values",
        "unsupported schema check: custom",
        "declared object schemas must reject unknown properties",
        "optional schemas are only supported as object properties",
        "unsupported schema type: date",
        "declared regular expressions cannot use global or sticky flags",
        "unsupported schema metadata: default",
    ]);
});
