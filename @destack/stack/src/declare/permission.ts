import { DeclarationName } from "@destack/package";
import { AccessName } from "@destack/sync";
import { PermissionReference } from "@destack/access";
import { defineSchema, Instant, schema } from "@destack/schema";
import { InstallationReference } from "./installation.ts";

/** An object declared in the stack's collections or selected by its persistent identifier. */
export const StackObjectReference = defineSchema(
    schema.union([
        schema.object({
            /** The object type, as its collection's object. */
            kind: AccessName,
            /** The stable key within that collection. */
            name: DeclarationName,
        }),
        schema.object({
            /** The persistent identifier with the object type as prefix. */
            id: schema.anyIdentifier(),
        }),
    ]),
);

/** A role declared by the stack or selected by its persistent identifier. */
export const StackRoleReference = defineSchema(
    schema.union([
        DeclarationName,
        schema.object({
            /** The existing role. */
            id: schema.identifier("role"),
        }),
    ]),
);

/** A named set of permissions, granted wherever the role is bound. */
export const RoleDefinition = defineSchema(
    schema.object({
        /** The role's purpose. */
        description: schema.string().min(1),
        /** The permissions the role grants beside those of the roles it includes. */
        permissions: schema.array(PermissionReference),
    }),
);

/** A subject related to an object in the space. */
export const StackSubject = defineSchema(
    schema.union([
        schema.object({
            /** A user. */
            user: schema.identifier("user"),
        }),
        schema.object({
            /** The members of a group of the space's account. */
            group: schema.identifier("group"),
        }),
        schema.object({
            /** Every member of the space's account. */
            accountMembers: schema.literal(true),
        }),
        schema.object({
            /** Every caller, signed in or anonymous. */
            anyone: schema.literal(true),
        }),
        schema.object({
            /** An installation in the destination space, configured or existing. */
            installation: InstallationReference,
        }),
        schema.object({
            /** A role of the space, included by a role through its includes relation. */
            role: StackRoleReference,
        }),
    ]),
);

/** The object, subject and lifetime of every space relationship. */
const SpaceRelated = schema.object({
    /** The related object, the space itself when absent. */
    object: StackObjectReference.exactOptional(),
    /** The related subject. */
    subject: StackSubject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: Instant.exactOptional(),
});

/** A relationship of a subject to the space or one of its objects. */
export const RelationshipDefinition = defineSchema(
    schema.union([
        SpaceRelated.extend({
            /** The role bound on the object. */
            role: StackRoleReference,
        }),
        SpaceRelated.extend({
            /** The object type's declared relation. */
            relation: AccessName,
        }),
    ]),
);
