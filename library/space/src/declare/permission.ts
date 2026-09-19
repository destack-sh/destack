import { ResourceName } from "@destack/resource";
import { defineSchema, identifier, schema } from "@destack/schema";
import { SpaceInstallationReference } from "./installation.ts";

/** An object selected from the resulting configuration or by its persistent identifier. */
export const SpacePermissionTarget = defineSchema(schema.union([
    schema.object({
        /** The configuration collection containing the object. */
        kind: schema.enum(["resource", "secret", "installation"]),
        /** The stable key within that collection. */
        name: ResourceName,
    }),
    schema.object({
        /** The persistent identifier checked against the role's space scope. */
        id: schema.union([
            identifier("resource"),
            identifier("secret"),
            identifier("installation"),
        ]),
    }),
]));

/** An exact API action granted within the space. */
export const SpacePermission = defineSchema(schema.object({
    /** The namespaced API resource type. */
    resource: schema.string().regex(/^[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*)+$/),
    /** The declared API action; wildcards are not accepted. */
    action: ResourceName,
    /** A selected object; absence grants all objects of the type within the space. */
    target: SpacePermissionTarget.optional(),
}));

/** A role whose generated grants are restricted to the configured space. */
export const SpaceRole = defineSchema(schema.object({
    /** The role's purpose. */
    description: schema.string().min(1),
    /** Additive permissions evaluated within the space. */
    permissions: schema.array(SpacePermission),
}));

/** A subject receiving a space-scoped role. */
export const SpaceSubject = defineSchema(schema.union([
    schema.object({
        /** The existing account membership. */
        membership: identifier("account-membership"),
    }),
    schema.object({
        /** The existing account group. */
        group: identifier("group"),
    }),
    schema.object({
        /** The existing workload service account in this space. */
        service: identifier("service-account"),
    }),
    schema.object({
        /** The external service account administered by the global account service. */
        accountService: identifier("service-account"),
    }),
    schema.object({
        /** The configured or existing installation in the destination space. */
        installation: SpaceInstallationReference,
        /** The installed package's workload name. */
        workload: ResourceName,
    }),
]));

/** Grant one configuration role to an existing member, group, or installed workload. */
export const SpaceRoleBinding = defineSchema(schema.object({
    /** The configured or existing role, checked against the destination space. */
    role: schema.union([
        ResourceName,
        schema.object({
            /** The existing role. */
            id: identifier("role"),
        }),
    ]),
    /** The subject receiving the role. */
    subject: SpaceSubject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: schema.number().int().nonnegative().optional(),
}));
