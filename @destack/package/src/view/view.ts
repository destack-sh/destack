import { defineSchema, schema } from "@destack/schema";
import { Entrypoint, PackageId } from "../package/package.ts";

/** A named frontend that clients can open independently. */
export const ViewDefinition = defineSchema(
    schema
        .object({
            /** The package export whose default export is the root component. */
            entrypoint: Entrypoint,
            /** Requested operations, granted explicitly by the application host. */
            permissions: schema
                .array(
                    schema
                        .object({
                            /** Package declaring the permission. */
                            packageId: PackageId,
                            /** Declared object type. */
                            type: schema.string().min(1),
                            /** Operation on that type. */
                            name: schema.string().min(1),
                        })
                        .strict(),
                )
                .optional(),
        })
        .strict(),
);

/** A named frontend that clients can open independently. */
export type ViewDefinition = schema.Infer<typeof ViewDefinition>;
