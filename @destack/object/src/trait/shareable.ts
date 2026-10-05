import {
    type AccessExpression,
    anyone,
    Explanation,
    permission,
    principal,
    Invitation,
    relation,
    Relationship,
    RelationshipCondition,
    through,
    union,
} from "@destack/access";
import { AccessName, Subject } from "@destack/sync";
import { Instant, schema } from "@destack/schema";
import { page } from "@destack/service/page";
import { type Method } from "../method/method.ts";
import {
    PageShape,
    type Procedure,
    type ReplayShape,
    type TargetShape,
} from "../method/procedure.ts";
import { field } from "../field/field.ts";
import {
    ObjectPermissions,
    type ObjectDefinition,
    type ObjectRelationInput,
    type ObjectType,
} from "../object/object.ts";
import type { Gated, Erasure, Trait } from "./trait.ts";

import { sharingMethods } from "../method/shareable.ts";
/** The roles, lowest first: viewing, commenting, editing and owning. */
const ROLES = ["viewer", "commenter", "editor", "owner"] as const;

/** The permissions the roles grant. */
const ROLE_PERMISSIONS = ["member", "read", "comment", "edit", "manage"] as const;

/** The lowest role granting each permission: membership is holding any role. */
const PERMISSIONS: { readonly [Name in RolePermission]: (typeof ROLES)[number] } = {
    member: "viewer",
    read: "viewer",
    comment: "commenter",
    edit: "editor",
    manage: "owner",
};

/** The fields relating a subject to an object, apart from its relation or role. */
export const SubjectShape = {
    /** The subject, subject set or wildcard. */
    subject: Subject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: Instant.exactOptional(),
    /** What a request must satisfy for the relationship to apply. */
    conditions: RelationshipCondition.exactOptional(),
};

/** The fields relating a subject to an object through exactly one relation or role, as callers send them. */
export const GrantShape = {
    /** The declared relation to grant. */
    relation: AccessName.exactOptional(),
    /** The role to bind. */
    role: schema.string().min(1).exactOptional(),
    ...SubjectShape,
};

/** A relationship an object's sharing methods grant: through exactly one declared relation or role. */
export const GrantInput = schema.union([
    schema.object({ ...SubjectShape, relation: AccessName }),
    schema.object({ ...SubjectShape, role: schema.string().min(1) }),
]);

/** The fields inviting a subject to a relationship: the caller, a principal, or a contact. */
export const InviteShape = {
    /** The relationship to invite to. */
    relationship: schema.object(GrantShape),
    /** Why the caller asks for or offers the relationship. */
    purpose: schema.string().min(1).max(1000).exactOptional(),
    /** When the invitation lapses in UTC epoch milliseconds, a week from now by default. */
    expiresAt: Instant.exactOptional(),
};

/** A relationship an object's sharing methods invite: through exactly one declared relation or role. */
export const InviteInput = schema.object({ ...InviteShape, relationship: GrantInput });

/** The field selecting one relationship. */
export const RelationshipShape = {
    /** The relationship's identifier. */
    relationshipId: schema.string().min(1),
};

/** The fields selecting what an explanation covers. */
export const ExplainShape = {
    /** The permission to explain, by default reading. */
    permission: AccessName.exactOptional(),
    /** The subject with the access to explain, by default the caller. */
    subject: Subject.exactOptional(),
};

/** The field selecting one invitation. */
export const InvitationShape = {
    /** The invitation's identifier. */
    invitationId: schema.string().min(1),
};

/** The methods sharing derives from an object's grant permission. */
export type ShareableMethodMap<Grant> = [Grant] extends [string]
    ? {
          readonly relationships: Method<{
              kind: "relationships";
              permission: string;
              mutates: false;
          }>;
          readonly grant: Method<{ kind: "grant"; permission: null; mutates: true }>;
          readonly revoke: Method<{ kind: "revoke"; permission: null; mutates: true }>;
          readonly invitations: Method<{ kind: "invitations"; permission: Grant; mutates: false }>;
          readonly invite: Method<{ kind: "invite"; permission: null; mutates: true }>;
          readonly accept: Method<{ kind: "accept"; permission: null; mutates: true }>;
          readonly withdraw: Method<{ kind: "withdraw"; permission: null; mutates: true }>;
          readonly explain: Method<{ kind: "explain"; permission: null; mutates: false }>;
      }
    : {};

/** A permission the roles grant: membership, read, comment, edit or manage. */
export type RolePermission = (typeof ROLE_PERMISSIONS)[number];

/** Sharing through the roles: owners, editors, commenters and viewers, shared by `manage`. */
export interface RolesDefinition {
    /** Whether anyone may read through the public relation, such as for publishing through a link, without being a member. */
    readonly isPublic?: true;
}

/** How callers share objects: through the roles, or through the type's own relations by a permission, and who sees the sharing. */
export type ShareableDefinition<Permission extends string = string> = (
    | Gated<Permission>
    | RolesDefinition
) & {
    /** The permission whose holders see the objects' relationships, `read` by default. */
    readonly read?: Permission;
};

/** The permissions a type shares through: the one granting relationships, and the one seeing them. */
export interface SharingGate {
    /** The permission whose holders grant and revoke relationships. */
    readonly grant: string;
    /** The permission whose holders see the relationships. */
    readonly read: string;
}

/** The permission whose holders share the objects: the definition's own, `manage` through the roles, undefined without sharing. */
export type SharingGateOf<Sharing> = [Sharing] extends [Gated<infer Permission>]
    ? Permission
    : [Sharing] extends [RolesDefinition]
      ? "manage"
      : undefined;

/** The permissions the roles grant when objects are shared through them, none otherwise. */
export type RolePermissionsOf<Sharing> = [Sharing] extends [Gated]
    ? never
    : [Sharing] extends [RolesDefinition]
      ? RolePermission
      : never;

/** The owner field the roles add to objects that are no scopes, none otherwise. */
export type RoleFieldsOf<Sharing, IsScope> = [RolePermissionsOf<Sharing>] extends [never]
    ? {}
    : IsScope extends true
      ? {}
      : { readonly owner: ReturnType<ReturnType<typeof field.subject>["caller"]> };

/** The roles of objects: owners, editors, commenters and viewers, each with the permissions of the roles below it. */
export const Roles = {
    /** Read the roles a definition shares through, absent when it shares otherwise or not at all. */
    of(definition: Pick<ObjectDefinition, "shareable">): RolesDefinition | undefined {
        const sharing = definition.shareable;

        return sharing === undefined || "by" in sharing ? undefined : sharing;
    },

    /** Read the permission whose holders share a definition's objects: its own, or `manage` through the roles. */
    gate(definition: Pick<ObjectDefinition, "shareable">): string | undefined {
        // share nothing without sharing
        const sharing = definition.shareable;
        if (sharing === undefined) {
            return undefined;
        }

        return "by" in sharing ? sharing.by : "manage";
    },

    /** Add the owner, the roles' relations and their permissions to a definition shared through the roles. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, "fields" | "relations" | "permissions"> {
        // keep a definition shared otherwise
        const roles = Roles.of(definition);
        if (roles === undefined) {
            return definition;
        }

        // refuse permission names alone
        const permissions = definition.permissions ?? {};
        if (ObjectPermissions.isList(permissions)) {
            throw new TypeError(
                `object ${definition.name} shares through the roles, so it declares its permissions as expressions`,
            );
        }
        const declared = permissions;

        // refuse the relations the roles define, and an owner field with no subject
        const owner = Object.entries(definition.fields ?? {}).find(
            ([property, candidate]) => candidate.relationAt(property) === "owner",
        )?.[1];
        const taken = [
            ...(owner !== undefined && owner.type !== "subject" ? ["owner"] : []),
            ...[...ROLES, ...(roles.isPublic === true ? ["public"] : [])].filter((name) =>
                Object.hasOwn(definition.relations ?? {}, name),
            ),
        ];
        if (taken.length > 0) {
            throw new TypeError(
                `object ${definition.name} shares through the roles, which define ${taken.join(", ")}`,
            );
        }

        // relate users, installations and group members through the roles
        const subjects = [
            principal.user,
            principal.installation,
            principal.group.members("member"),
        ];
        const relations: Record<string, ObjectRelationInput> = {
            ...(definition.isScope === true ? { owner: { subjects } } : {}),
            editor: { subjects },
            commenter: { subjects },
            viewer: { subjects },
            ...(roles.isPublic === true ? { public: { subjects: [anyone.all()] } } : {}),
        };

        return {
            ...definition,
            fields:
                definition.isScope === true || owner !== undefined
                    ? (definition.fields ?? {})
                    : { owner: field.subject().caller(), ...definition.fields },
            relations: { ...definition.relations, ...relations },
            permissions: { ...declared, ...Roles.permissions(definition, declared) },
        };
    },

    /** Derive each permission: the roles granting it on the object, the same permission of its parent or enclosing scopes, or what the definition adds. */
    permissions(
        definition: ObjectDefinition,
        declared: Readonly<Record<string, AccessExpression>>,
    ): Readonly<Record<string, AccessExpression>> {
        // grant each permission through the roles at or above its own, highest first
        const names = ROLE_PERMISSIONS;
        const holders = (name: RolePermission) =>
            ROLES.slice(ROLES.indexOf(PERMISSIONS[name]))
                .toReversed()
                .map((role) => relation(role));

        // widen each by what the definition adds, and reading by the public relation
        const isPublic = Roles.of(definition)?.isPublic === true;
        const added = (name: RolePermission) => [
            ...(name === "read" && isPublic ? [relation("public")] : []),
            ...(declared[name] === undefined ? [] : [declared[name]]),
        ];

        // inherit from each enclosing scope shared through the roles
        const enclosing = (name: RolePermission) =>
            Roles.scopes(definition).map((scope) => through(scope.name, name));

        // inherit the roles of every ancestor of a tree and of the enclosing scopes
        const parent = definition.nested?.in;
        if (parent === "self") {
            return Object.fromEntries(
                names.flatMap((name) => [
                    [`${name}-direct`, union(...holders(name), ...added(name))],
                    [
                        name,
                        union(
                            permission(`${name}-direct`),
                            through("parent", `${name}-direct`, true),
                            ...enclosing(name),
                        ),
                    ],
                ]),
            );
        }

        // inherit from the parent, or else from the scopes
        const inherited = (name: RolePermission) =>
            parent !== undefined && parent !== "any" ? [through("parent", name)] : enclosing(name);

        return Object.fromEntries(
            names.map((name) => [
                name,
                union(...holders(name), ...inherited(name), ...added(name)),
            ]),
        );
    },

    /** List the scope types enclosing a definition's objects that share through the roles. */
    scopes(definition: ObjectDefinition): readonly ObjectType[] {
        return [definition.scope ?? []]
            .flat()
            .filter(
                (scope): scope is ObjectType =>
                    typeof scope !== "string" && scope.roles !== undefined,
            );
    },
};

/** Objects callers share through relationships and invitations. */
export const shareable: Trait<SharingGate> = {
    key: "shareable",
    isDurable: true,
    options: (definition) => {
        const grant = Roles.gate(definition);

        return grant === undefined
            ? undefined
            : { grant, read: definition.shareable?.read ?? "read" };
    },
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => sharingMethods(options),
};

/** The procedures sharing derives. */
export type ShareableProcedures<Object extends ObjectType> = {
    explain: Procedure<
        schema.Object<TargetShape<Object> & typeof ExplainShape>,
        typeof Explanation
    >;
    relationships: Procedure<
        schema.Object<TargetShape<Object> & typeof PageShape>,
        ReturnType<typeof page<typeof Relationship.schema>>
    >;
    grant: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof GrantShape>,
        typeof Relationship.schema
    >;
    revoke: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof RelationshipShape>,
        schema.Object<{}>
    >;
    invitations: Procedure<
        schema.Object<TargetShape<Object> & typeof PageShape>,
        ReturnType<typeof page<typeof Invitation.schema>>
    >;
    invite: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof InviteShape>,
        typeof Invitation.schema
    >;
    accept: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof InvitationShape>,
        typeof Relationship.schema
    >;
    withdraw: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof InvitationShape>,
        schema.Object<{}>
    >;
};
