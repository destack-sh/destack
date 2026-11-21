import { schema } from "@destack/schema";

/** The path of a declaration, a resource or a target inside one. */
export const Address = Object.assign(schema.string().regex(/^[^/\s]+(?:\/[^/\s]+)*$/u), {
    /** Join address segments, such as a declaration's kind and name. */
    join(...segments: readonly string[]): string {
        return segments.join("/");
    },

    /** Split an address into its segments. */
    split(address: string): string[] {
        return address.split("/");
    },
});
/** The path of a declaration, a resource or a target inside one. */
export type Address = schema.Infer<typeof Address>;
