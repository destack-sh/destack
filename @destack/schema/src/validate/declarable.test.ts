import { z } from "zod";
import { expect, test } from "@destack/test";
import { requireDeclarable } from "./declarable.ts";

test("accept schemas of JSON values with exportable rules", () => {
    const declared = z.strictObject({
        name: z
            .string()
            .min(1)
            .max(63)
            .regex(/^[a-z]+$/u),
        count: z.number().int().nonnegative(),
        tags: z.array(z.enum(["a", "b"])).exactOptional(),
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
            z.array(z.string().exactOptional()),
            z.date(),
            z.string().regex(/a/gu),
            z.string().meta({ default: "x" }),
        ].map((declared) => {
            try {
                requireDeclarable(declared);

                return "accepted";
            } catch (error) {
                if (!(error instanceof TypeError)) {
                    throw error;
                }

                return error.message;
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
