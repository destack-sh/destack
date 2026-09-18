import { defineSchema, schema } from "@destack/schema";
import { DependencyName, Package } from "../package/package.ts";
import { Digest } from "../file/file.ts";

/** An exact dependency selected during package resolution. */
export const DependencyResolution = defineSchema(schema.union([
    schema.object({
        /** The Destack registry format. */
        kind: schema.literal("destack"),
        /** The resolved package and release. */
        package: Package,
        /** The digest of its immutable build manifest. */
        manifest: Digest,
    }),
    schema.object({
        /** The npm registry format. */
        kind: schema.literal("npm"),
        /** The resolved package and release, including npm aliases. */
        package: Package.extend({ name: DependencyName }),
        /** The resolved tarball URL. */
        tarball: schema.string().regex(/^https?:\/\/[^\s]+$(?![\s\S])/),
        /** The registry's Subresource Integrity expression. */
        integrity: schema.string().min(1),
    }),
]));
/** An exact dependency selected during package resolution. */
export type DependencyResolution = schema.Infer<typeof DependencyResolution>;
