import { defineSchema, schema } from "@destack/schema";

/** The path of a declaration, a resource or a target inside one. */
export const Address = Object.assign(
    defineSchema(schema.string().regex(/^[^/\s]+(?:\/[^/\s]+)*$/u)),
    {
        /** Join address segments, such as a declaration's kind and name. */
        join(...segments: readonly string[]): string {
            return segments.join("/");
        },
    },
);
/** The path of a declaration, a resource or a target inside one. */
export type Address = schema.Infer<typeof Address>;
