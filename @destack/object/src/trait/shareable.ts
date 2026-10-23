import {
    type AccessExpression,
    anyone,
    Explanation,
    permission,
    principal,
    Invitation,
    relation,
    Relationship,
    through,
    union,
} from "@destack/access";
import { AccessName, Subject } from "@destack/sync";
import { Digest, Instant, schema } from "@destack/schema";
import { type Method } from "../method/method.ts";
import {
    PageFields,
    type Procedure,
    type ReplayFields,
    type TargetFields,
    ListPage,
} from "../method/procedure.ts";
import { field } from "../field/field.ts";
import { ObjectError } from "../error/error.ts";
import {
    ObjectPermissions,
    type ObjectDefinition,
    type ObjectRelationInput,
    type ObjectType,
} from "../object/object.ts";
import type { Gated, Erasure, Trait } from "./trait.ts";

import { ShareableMethod } from "../method/shareable.ts";
/** The roles, lowest first: viewer, commenter, editor and owner. */
const ROLES = ["viewer", "commenter", "editor", "owner"] as const;

/** The permissions the roles grant. */
const ROLE_PERMISSIONS = ["member", "read", "comment", "write", "manage"] as const;

/** The lowest role granting each permission: membership is having any role. */
const PERMISSIONS: { readonly [Name in RolePermission]: (typeof ROLES)[number] } = {
    member: "viewer",
    read: "viewer",
    comment: "commenter",
    write: "editor",
    manage: "owner",
};

/** The fields relating a subject to an object, apart from its relation or role. */
export const SubjectFields = {
    /** The subject, subject set or wildcard. */
    subject: Subject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: Instant.exactOptional(),
    /** The digest of the link secret a request must present, for a share link. */
    linkSecret: Digest.exactOptional(),
};

/** The fields relating a subject to an object through one relation or role, as sent. */
export const GrantFields = {
    /** The declared relation to grant. */
    relation: AccessName.exactOptional(),
    /** The role to bind. */
    role: schema.string().min(1).exactOptional(),
    ...SubjectFields,
};

/** A relationship sharing methods grant: through exactly one declared relation or role. */
export const GrantInput = schema.union([
    schema.object({ ...SubjectFields, relation: AccessName }),
    schema.object({ ...SubjectFields, role: schema.string().min(1) }),
]);

/** Read a grant through exactly one declared relation or role, refusing both or neither. */
export function readGrant(input: unknown): schema.Infer<typeof GrantInput> {
    const grant = GrantInput.safeParse(input);
    if (!grant.success) {
        throw ObjectError.of("INVALID_CALL", "a grant must have exactly one relation or role");
    }

    return grant.data;
}

/** The fields inviting a subject to a relationship: the caller, a principal, or a contact. */
export const InviteFields = {
    /** The relationship to invite to. */
    relationship: schema.object(GrantFields),
    /** Why the caller asks for or offers the relationship. */
    purpose: schema.string().min(1).max(1000).exactOptional(),
    /** When the invitation lapses in UTC epoch milliseconds, a week from now by default. */
    expiresAt: Instant.exactOptional(),
};

/** A relationship sharing methods invite: through exactly one declared relation or role. */
export const InviteInput = schema.object({ ...InviteFields, relationship: GrantInput });

/** The field selecting one relationship. */
export const RelationshipFields = {
    /** The relationship's identifier. */
    relationshipId: schema.string().min(1),
};

/** The fields selecting what an explanation covers. */
export const ExplainFields = {
    /** The permission to explain, by default reading. */
    permission: AccessName.exactOptional(),
    /** The subject with the access to explain, by default the caller. */
    subject: Subject.exactOptional(),
};

/** The field selecting one invitation. */
export const InvitationFields = {
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
          readonly grant: Method<{ kind: "grant"; permission: string; mutates: true }>;
          readonly revoke: Method<{ kind: "revoke"; permission: string; mutates: true }>;
          readonly invitations: Method<{ kind: "invitations"; permission: Grant; mutates: false }>;
          readonly invite: Method<{ kind: "invite"; permission: string; mutates: true }>;
          readonly accept: Method<{ kind: "accept"; permission: string; mutates: true }>;
          readonly withdraw: Method<{ kind: "withdraw"; permission: string; mutates: true }>;
          readonly explain: Method<{ kind: "explain"; permission: string; mutates: false }>;
      }
    : {};

/** A permission the roles grant: membership, read, comment, edit or manage. */
export type RolePermission = (typeof ROLE_PERMISSIONS)[number];

/** Sharing through the roles: owners, editors, commenters and viewers, shared by `manage`. */
export interface RolesDefinition {
    /** Whether anyone may read through the public relation without membership, as for links. */
    readonly isPublic?: true;
}

/** How callers share objects: through roles, or separate relations by permission, and who sees. */
export type ShareableDefinition<Permission extends string = string> = (
    | Gated<Permission>
    | RolesDefinition
) & {
    /** The permission whose holders see the objects' relationships, `read` by default. */
    readonly read?: Permission;
};

/** The permissions of sharing: the one granting relationships, and the one seeing them. */
export interface SharingGate {
    /** The permission whose holders grant and revoke relationships. */
    readonly grant: string;
    /** The permission whose holders see the relationships. */
    readonly read: string;
}

/** The sharing permission: the definition's, `manage` through roles, or undefined. */
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

/** The owner field the roles add to objects that are not scopes, none otherwise. */
export type RoleFieldsOf<Sharing, IsScope> = [RolePermissionsOf<Sharing>] extends [never]
    ? {}
    : IsScope extends true
      ? {}
      : { readonly owner: ReturnType<ReturnType<typeof field.subject>["caller"]> };

/** The object roles: owners, editors, commenters and viewers, each with lower roles' rights. */
export const Roles = {
    /** Read a definition's sharing roles, absent if it shares otherwise or not at all. */
    of(definition: Pick<ObjectDefinition, "shareable">): RolesDefinition | undefined {
        const sharing = definition.shareable;

        return sharing === undefined || "by" in sharing ? undefined : sharing;
    },

    /** Read a definition's sharing permission: its declared one, or `manage` through the roles. */
    gate(definition: Pick<ObjectDefinition, "shareable">): string | undefined {
        // share nothing without sharing
        const sharing = definition.shareable;
        if (sharing === undefined) {
            return undefined;
        }

        return "by" in sharing ? sharing.by : "manage";
    },

    /** Add the owner, the roles' relations and permissions to a role-shared definition. */
    expand<Definition extends ObjectDefinition>(
        definition: Definition,
    ): Erasure<Definition, "fields" | "relations" | "permissions"> {
        // leave a definition shared otherwise
        const roles = Roles.of(definition);
        if (roles === undefined) {
            return definition;
        }

        // refuse bare permission names
        const permissions = definition.permissions ?? {};
        if (ObjectPermissions.isList(permissions)) {
            throw new TypeError(
                `object ${definition.name} shares through roles, so it must declare its permissions as expressions`,
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
                `object ${definition.name} shares through roles and cannot declare ${taken.join(", ")}`,
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

    /** Derive each permission: granting roles on the object, the parent's or scopes' same one. */
    permissions(
        definition: ObjectDefinition,
        declared: Readonly<Record<string, AccessExpression>>,
    ): Readonly<Record<string, AccessExpression>> {
        // grant each permission through the roles at or above its role, highest first
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
    storages: ["durable"],
    options: (definition) => {
        // derive no sharing methods for a partitioned scope, shared through its relationships
        const grant = Roles.gate(definition);

        return grant === undefined || definition.partition === true
            ? undefined
            : { grant, read: definition.shareable?.read ?? "read" };
    },
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => ShareableMethod.methods(options),
};

/** The procedures sharing derives. */
export type ShareableProcedures<Object extends ObjectType> = {
    explain: Procedure<
        schema.Object<TargetFields<Object> & typeof ExplainFields>,
        typeof Explanation
    >;
    relationships: Procedure<
        schema.Object<TargetFields<Object> & typeof PageFields>,
        ReturnType<typeof ListPage.schema<typeof Relationship.schema>>
    >;
    grant: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object> & typeof GrantFields>,
        typeof Relationship.schema
    >;
    revoke: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object> & typeof RelationshipFields>,
        schema.Object<{}>
    >;
    invitations: Procedure<
        schema.Object<TargetFields<Object> & typeof PageFields>,
        ReturnType<typeof ListPage.schema<typeof Invitation.schema>>
    >;
    invite: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object> & typeof InviteFields>,
        typeof Invitation.schema
    >;
    accept: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object> & typeof InvitationFields>,
        typeof Relationship.schema
    >;
    withdraw: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object> & typeof InvitationFields>,
        schema.Object<{}>
    >;
};
