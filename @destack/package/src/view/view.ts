import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "../definition/package.ts";
import { PackagePath } from "../file/file.ts";

/** A view a browser output compiles, as manifests describe it. */
export const ViewDescription = defineSchema(
    schema
        .object({
            /** The emitted chunk mounting the view. */
            entrypoint: PackagePath,
            /** The permissions the view requests. */
            permissions: schema.array(
                schema
                    .object({
                        /** The package declaring the permission. */
                        packageId: PackageId,
                        /** The object type. */
                        type: schema.string().min(1),
                        /** The permission on that type. */
                        name: schema.string().min(1),
                    })
                    .strict(),
            ),
        })
        .strict(),
);

/** A view a browser output compiles, as manifests describe it. */
export type ViewDescription = schema.Infer<typeof ViewDescription>;
