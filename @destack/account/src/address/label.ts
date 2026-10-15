import { NOT_XN } from "@destack/runtime";
import type { FieldCondition } from "@destack/db";
import { schema } from "@destack/schema";

/** The characters of a label, as a character class: lowercase letters, digits and hyphens. */
const CHARACTERS = "a-z0-9-";

/** A name that composes into one DNS label: lowercase letters, digits and single inner hyphens. */
export const Label = {
    /** Validate a label of at most a length. */
    schema(length: number) {
        return schema
            .string()
            .max(length)
            .regex(
                /^[a-z0-9]+(?:-[a-z0-9]+)*$/u,
                "labels are lowercase letters, digits and single inner hyphens",
            )
            .regex(NOT_XN, "labels other than xn, which starts punycode");
    },

    /** Check a label of at most a length, as a column's condition. */
    condition(length: number): FieldCondition {
        return {
            length: { gte: 1, lte: length },
            ne: "xn",
            characters: CHARACTERS,
            NOT: { OR: [{ startsWith: "-" }, { endsWith: "-" }, { like: "%--%" }] },
        };
    },
};
