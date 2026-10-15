import { Package } from "@destack/package";
import { defineSchema, Digest, schema } from "@destack/schema";

/** The stack a space has from its creation, which the machine receiving the space applies. */
export const SpaceStack = defineSchema(
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
/** The stack a space has from its creation. */
export type SpaceStack = schema.Infer<typeof SpaceStack>;
