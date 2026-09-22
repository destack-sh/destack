import { ResourceName } from "../resource/index.ts";
import { defineSchema, identifier, schema } from "@destack/schema";

/** A provisioned resource in a space. */
export const ResourceReference = defineSchema(
    schema.object({
        /** The space containing the resource. */
        space: identifier("space"),
        /** The persistent resource identifier. */
        resource: identifier("resource"),
    }),
);
/** A provisioned resource in a space. */
export type ResourceReference = schema.Infer<typeof ResourceReference>;

/** Bind one installation's declaration to a provisioned resource. */
export const Binding = defineSchema(
    schema.object({
        /** The installation containing the declaration. */
        installation: identifier("installation"),
        /** The package declaring the resource, including imported stack packages. */
        package: identifier("package"),
        /** The declaration name in its package. */
        name: ResourceName,
        /** The resource selected by the host. */
        target: ResourceReference,
    }),
);
/** A resource binding, separate from authorization. */
export type Binding = schema.Infer<typeof Binding>;
