import { Package } from "@destack/package";
import { defineSchema, Digest, schema } from "@destack/schema";

/** Where a space's stack comes from: a stack release's export and its setting values. */
export const StackSource = defineSchema(
    schema.object({
        /** The stack package. */
        package: Package,
        /** The digest of the stack package's build manifest. */
        manifest: Digest,
        /** The export declaring the space. */
        export: schema.string().min(1),
        /** The setting values the stack places in the space, by `<package>/<name>`. */
        settings: schema.record(schema.string().min(1), schema.json()),
    }),
);
/** Where a space's stack comes from. */
export type StackSource = schema.Infer<typeof StackSource>;

/** The kind of a space, which decides its stack: a residency home, a person's home, or other. */
export const SpaceKind = defineSchema(schema.enum(["residency", "person", "space"]));
/** The kind of a space. */
export type SpaceKind = schema.Infer<typeof SpaceKind>;
