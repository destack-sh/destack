import { defineSchema, schema } from "@destack/schema";
import { Package } from "@destack/package";
import { ResourceDescription, ResourcePlacement, ResourceRetention } from "./declaration.ts";

/** The provider code of resources lent from connections. */
export const CONNECTION_PROVIDER = "connection";

/** The header in which an outbound call to a lent resource gives its connection. */
export const CONNECTION_HEADER = "Destack-Connection";

/** A resource of any kind a stack declares. */
export const ResourceDefinition = defineSchema(
    schema.object({
        /** The resource declaration imported from its declaring package. */
        declaration: ResourceDescription.extend({
            /** The package declaring the resource. */
            package: Package,
        }),
        /** The reference of a resource no machine provisions, such as a service's address. */
        reference: schema.string().min(1).exactOptional(),
        /** The connection whose credential the machine lends to every call. */
        connection: schema.identifier("connection").exactOptional(),
        /** What authorised removal does to the resource's contents. */
        retention: ResourceRetention,
        /** Provider placement, absent when the machine selects it. */
        placement: ResourcePlacement.exactOptional(),
        /** User-defined labels. */
        tags: schema.record(schema.string().min(1), schema.string()),
    }),
);
/** A resource of any kind a stack declares. */
export type ResourceDefinition = schema.Infer<typeof ResourceDefinition>;
