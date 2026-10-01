import { defineSchema, schema } from "@destack/schema";

/** The path of a part of a declaration or resource, such as `database/main/table/note/column/title`. */
export const Address = Object.assign(
    defineSchema(schema.string().regex(/^[^/\s]+(?:\/[^/\s]+)*$(?![\s\S])/)),
    {
        /** Join address parts, such as a declaration's kind and name. */
        join(...parts: readonly string[]): string {
            return parts.join("/");
        },
    },
);
/** The path of a part of a declaration or resource. */
export type Address = schema.Infer<typeof Address>;
