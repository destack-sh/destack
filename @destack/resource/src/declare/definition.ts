import { defineSchema, schema } from "@destack/schema";
import { Package } from "@destack/package";
import {
    ResourceDescription,
    ResourcePlacement,
    ResourceReference,
    ResourceRetention,
} from "./declaration.ts";

/** A resource of any kind a stack declares: its package's declaration, retention and placement, or an existing resource. */
export const ResourceDefinition = defineSchema(
    schema.object({
        /** The resource declaration imported from its declaring package. */
        declaration: ResourceDescription.extend({
            /** The package declaring the resource. */
            package: Package,
        }),
        /** An existing resource to adopt, subject to ownership and residency checks. */
        adopt: ResourceReference.exactOptional(),
        /** The reference of a resource no host provisions, such as a service's address, connected by the placement's provider. */
        reference: schema.string().min(1).exactOptional(),
        /** What authorised removal does to the resource's contents. */
        retention: ResourceRetention,
        /** Provider placement, absent when selected by the host. */
        placement: ResourcePlacement.exactOptional(),
        /** User-defined labels. */
        tags: schema.record(schema.string().min(1), schema.string()),
    }),
);
/** A resource of any kind a stack declares. */
export type ResourceDefinition = schema.Infer<typeof ResourceDefinition>;
