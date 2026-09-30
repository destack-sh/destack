import { ResourceDescription, ResourceReference } from "@destack/resource";
import { DeclarationName, Package } from "@destack/package";
import { AccessName } from "@destack/access";
import { defineSchema, identifier, schema } from "@destack/schema";

/** A resource created by the configuration or explicitly adopted into its administration. */
export const SpaceResource = defineSchema(
    schema.object({
        /** The resource declaration imported from its declaring package. */
        declaration: ResourceDescription.extend({
            /** The package declaring the resource. */
            package: Package,
        }),
        /** An existing resource to adopt, subject to ownership and residency checks. */
        adopt: ResourceReference.optional(),
        /** The reference of a resource no host provisions, such as a service's address, connected by the placement's provider. */
        reference: schema.string().min(1).optional(),
        /** Whether authorised removal retains or destroys the resource's contents. */
        retention: schema.enum(["retain", "delete"]),
        /** Provider placement, absent when selected by the host. */
        placement: schema
            .object({
                /** The provider adapter. */
                provider: schema.string().min(1),
                /** The location code accepted by the provider adapter. */
                location: schema.string().min(1).optional(),
                /** The host administering the resource. */
                host: identifier("host").optional(),
            })
            .optional(),
        /** User-defined labels. */
        tags: schema.record(schema.string().min(1), schema.string()),
    }),
);
/** A resource administered through a configuration. */
export type SpaceResource = schema.Infer<typeof SpaceResource>;

/** A bindable object a binding targets: one the stack declares by key, or an existing one in the destination space. */
export const SpaceBindingTarget = defineSchema(
    schema.union([
        schema.object({
            /** The bindable object type, such as resource or secret. */
            type: AccessName,
            /** The object's key in this configuration. */
            name: DeclarationName,
        }),
        schema.object({
            /** The bindable object type, such as resource or secret. */
            type: AccessName,
            /** The existing object; application verifies it lives in the destination space. */
            id: schema.string().min(1),
        }),
    ]),
);
/** A bindable object a binding targets. */
export type SpaceBindingTarget = schema.Infer<typeof SpaceBindingTarget>;

/** Bind one of a package's declarations to an object the space has: a resource, a secret, or any other bindable object. */
export const SpaceBinding = defineSchema(
    schema.object({
        /** The bound object. */
        target: SpaceBindingTarget,
        /** An exact version or generation to run with, else the target's current one. */
        version: schema.number().int().positive().optional(),
        /** The state the declaration requires of the target, such as a database's tables. */
        state: schema.record(schema.string(), schema.json()),
    }),
);
/** A binding of one declaration to an object the space has. */
export type SpaceBinding = schema.Infer<typeof SpaceBinding>;
