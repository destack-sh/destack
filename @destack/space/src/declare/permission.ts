import { DeclarationName } from "@destack/package";
import { AccessName, PermissionReference } from "@destack/access";
import { defineSchema, identifier, schema } from "@destack/schema";
import { SpaceInstallationReference } from "./installation.ts";

/** An object declared in the stack's collections or selected by its persistent identifier. */
export const SpaceObjectReference = defineSchema(
    schema.union([
        schema.object({
            /** The object type, named as its collection's object. */
            kind: AccessName,
            /** The stable key within that collection. */
            name: DeclarationName,
        }),
        schema.object({
            /** The persistent identifier with the object type as prefix. */
            id: schema.string().regex(/^[a-z][a-z0-9-]*-[0-9a-f-]+$(?![\s\S])/),
        }),
    ]),
);

/** A role declared by the stack or selected by its persistent identifier. */
export const SpaceRoleReference = defineSchema(
    schema.union([
        DeclarationName,
        schema.object({
            /** The existing role. */
            id: identifier("role"),
        }),
    ]),
);

/** A named set of permissions, granted wherever the role is bound. */
export const SpaceRole = defineSchema(
    schema.object({
        /** The role's purpose. */
        description: schema.string().min(1),
        /** The permissions the role grants; relationships relating it to included roles add theirs. */
        permissions: schema.array(PermissionReference),
    }),
);

/** A subject related to an object in the space. */
export const SpaceSubject = defineSchema(
    schema.union([
        schema.object({
            /** A user. */
            user: identifier("user"),
        }),
        schema.object({
            /** The members of a group of the space's account. */
            group: identifier("group"),
        }),
        schema.object({
            /** Every member of the space's account. */
            accountMembers: schema.literal(true),
        }),
        schema.object({
            /** An installation in the destination space, configured or existing. */
            installation: SpaceInstallationReference,
        }),
        schema.object({
            /** A service account of the space's account, such as an external integration. */
            serviceAccount: identifier("service-account"),
        }),
        schema.object({
            /** A role of the space, included by a role through its includes relation. */
            role: SpaceRoleReference,
        }),
    ]),
);

/** The object, subject and lifetime of every space relationship. */
const SpaceRelated = schema.object({
    /** The related object; the space itself when absent. */
    object: SpaceObjectReference.optional(),
    /** The related subject. */
    subject: SpaceSubject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: schema.number().int().nonnegative().optional(),
});

/** Relate a subject to an object of the space, or to the space itself, through a role or a declared relation. */
export const SpaceRelationship = defineSchema(
    schema.union([
        SpaceRelated.extend({
            /** The role bound on the object. */
            role: SpaceRoleReference,
        }),
        SpaceRelated.extend({
            /** The object type's declared relation. */
            relation: AccessName,
        }),
    ]),
);
