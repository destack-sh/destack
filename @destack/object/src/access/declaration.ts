import { PermissionReference } from "@destack/access";
import { DeclarationName } from "@destack/package";
import { Instant, schema } from "@destack/schema";
import { AccessName } from "@destack/sync";

/** The references of records a stack declares: by name, or an existing one by identifier. */
export const StackReference = {
    /** Build the references of one kind's records. */
    of<const Kind extends string>(kind: Kind) {
        return schema.union([
            DeclarationName,
            schema.object({
                /** The existing record. */
                id: schema.identifier(kind),
            }),
        ]);
    },
};

/** An object declared in the stack's collections or selected by its persistent identifier. */
export const StackObjectReference = schema.union([
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
]);

/** A role declared by the stack or selected by its persistent identifier. */
export const StackRoleReference = StackReference.of("role");

/** A named set of permissions, granted wherever the role is bound. */
export const RoleDefinition = schema.object({
    /** The role's purpose. */
    description: schema.string().min(1),
    /** The permissions the role grants beside those of the roles it includes. */
    permissions: schema.array(PermissionReference),
});

/** A subject related to an object in the scope. */
export const StackSubject = schema.union([
    schema.object({
        /** A user. */
        user: schema.identifier("user"),
    }),
    schema.object({
        /** The members of a group of the scope's account. */
        group: schema.identifier("group"),
    }),
    schema.object({
        /** Every member of the scope's account. */
        accountMembers: schema.literal(true),
    }),
    schema.object({
        /** Every caller, signed in or anonymous. */
        anyone: schema.literal(true),
    }),
    schema.object({
        /** An installation in the scope, declared or existing. */
        installation: StackReference.of("installation"),
    }),
    schema.object({
        /** A role of the scope, included by a role through its includes relation. */
        role: StackRoleReference,
    }),
]);

/** The object, subject and lifetime of every relationship a stack declares. */
const ScopeRelated = schema.object({
    /** The related object, the scope itself when absent. */
    object: StackObjectReference.exactOptional(),
    /** The related subject. */
    subject: StackSubject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: Instant.exactOptional(),
});

/** A relationship of a subject to the scope or one of its objects. */
export const RelationshipDefinition = schema.union([
    ScopeRelated.extend({
        /** The role bound on the object. */
        role: StackRoleReference,
    }),
    ScopeRelated.extend({
        /** The object type's declared relation. */
        relation: AccessName,
    }),
]);
