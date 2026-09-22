import { ResourceDeclaration, ResourceName, ResourceReference } from "@destack/resource";
import { defineSchema, identifier, schema } from "@destack/schema";

/** A resource created by the configuration or explicitly adopted into its administration. */
export const SpaceResource = defineSchema(
    schema.object({
        /** The resource declaration imported from its domain library. */
        declaration: ResourceDeclaration,
        /** An existing resource to adopt, subject to ownership and residency checks. */
        adopt: ResourceReference.optional(),
        /** Whether authorised removal retains or destroys the resource contents. */
        retention: schema.enum(["retain", "delete"]),
        /** Provider placement, absent when selected by the host. */
        placement: schema
            .object({
                /** The provider adapter. */
                provider: schema.string().min(1),
                /** The provider region registered by the platform. */
                region: identifier("region").optional(),
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

/** Secret metadata declared in source, with values supplied through the vault API. */
export const SpaceSecret = defineSchema(
    schema.object({
        /** The configuration resource containing the secret. */
        vault: ResourceName,
        /** The name within the vault. */
        name: ResourceName,
    }),
);
/** A source-declared secret without a value. */
export type SpaceSecret = schema.Infer<typeof SpaceSecret>;

/** Select a declared or existing resource in the destination space. */
export const SpaceResourceBinding = defineSchema(
    schema.union([
        schema.object({
            /** The resource key in this configuration. */
            resource: ResourceName,
        }),
        schema.object({
            /** The existing resource; application verifies its destination space. */
            external: ResourceReference,
        }),
    ]),
);

/** Select a configuration secret or an explicitly authorised existing secret. */
export const SpaceSecretBinding = defineSchema(
    schema.object({
        /** The selected secret. */
        target: schema.union([
            schema.object({
                /** The secret key in this configuration. */
                secret: ResourceName,
            }),
            schema.object({
                /** The destination space containing the existing secret. */
                space: identifier("space"),
                /** The existing secret. */
                secret: identifier("secret"),
            }),
        ]),
        /** Exact version; absence selects the current version at access time. */
        version: schema.number().int().positive().optional(),
    }),
);
