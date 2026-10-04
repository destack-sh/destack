import {
    type AccessExpression,
    anyone,
    Explanation,
    permission,
    principal,
    Proposal,
    relation,
    Relationship,
    RelationshipCondition,
    through,
    union,
} from "@destack/access";
import { AccessName, Subject } from "@destack/sync";
import { and, eq, TABLE, type Row } from "@destack/db";
import { Instant, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Page, page } from "@destack/service/page";
import { Call } from "../method/call.ts";
import { Step } from "../method/step.ts";
import type * as sync from "@destack/sync";
import {
    defineMethod,
    type Method,
    type MethodKind,
    type MethodConfiguration,
} from "../method/method.ts";
import {
    Empty,
    PageShape,
    type MethodProcedure,
    type ObjectSchema,
    type Procedure,
    type ReplayShape,
    type Route,
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
const SubjectShape = {
    /** The subject, subject set or wildcard. */
    subject: Subject,
    /** Optional expiry in UTC epoch milliseconds. */
    expiresAt: Instant.exactOptional(),
    /** What a request must satisfy for the relationship to apply. */
    conditions: RelationshipCondition.exactOptional(),
};

/** The fields relating a subject to an object through exactly one relation or role, as callers send them. */
const GrantShape = {
    /** The declared relation to grant. */
    relation: AccessName.exactOptional(),
    /** The role to bind. */
    role: schema.string().min(1).exactOptional(),
    ...SubjectShape,
};

/** A relationship an object's sharing methods grant: through exactly one declared relation or role. */
const GrantInput = schema.union([
    schema.object({ ...SubjectShape, relation: AccessName }),
    schema.object({ ...SubjectShape, role: schema.string().min(1) }),
]);

/** The fields proposing a relationship to its subject: the caller, a principal, or a contact. */
const ProposeShape = {
    /** The relationship to propose. */
    relationship: schema.object(GrantShape),
    /** Why the caller asks for or offers the relationship. */
    purpose: schema.string().min(1).max(1000).exactOptional(),
    /** When the proposal lapses in UTC epoch milliseconds, a week from now by default. */
    expiresAt: Instant.exactOptional(),
};

/** A relationship an object's sharing methods propose: through exactly one declared relation or role. */
const ProposeInput = schema.object({ ...ProposeShape, relationship: GrantInput });

/** The field selecting one relationship. */
const RelationshipShape = {
    /** The relationship's identifier. */
    relationshipId: schema.string().min(1),
};

/** The fields selecting what an explanation covers. */
const ExplainShape = {
    /** The permission to explain, by default reading. */
    permission: AccessName.exactOptional(),
    /** The subject with the access to explain, by default the caller. */
    subject: Subject.exactOptional(),
};

/** The field selecting one proposal. */
const ProposalShape = {
    /** The proposal's identifier. */
    proposalId: schema.string().min(1),
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
          readonly proposals: Method<{ kind: "proposals"; permission: Grant; mutates: false }>;
          readonly propose: Method<{ kind: "propose"; permission: null; mutates: true }>;
          readonly accept: Method<{ kind: "accept"; permission: null; mutates: true }>;
          readonly decline: Method<{ kind: "decline"; permission: null; mutates: true }>;
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
interface SharingPermissions {
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

/**
 * The roles of objects: owners, editors, commenters and viewers, each with the permissions of the roles below it.
 *
 * An object inherits each permission from its parent, or from the scopes enclosing it shared through the roles too, as files inherit from folders.
 */
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
        return [definition.scope]
            .flat()
            .filter(
                (scope): scope is ObjectType =>
                    typeof scope !== "string" && scope.roles !== undefined,
            );
    },
};

/** Objects callers share through relationships and proposals. */
export const shareable: Trait<SharingPermissions> = {
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
    methods: ({ grant, read }) => ({
        relationships: sharingMethod(
            "relationships",
            read,
            { method: "GET", path: "/{id}/relationships" },
            (shapes) => shapes.target.extend(PageShape),
            page(Relationship.schema),
            async (call) => {
                // page the object's relationships by identifier
                const listing = sharingPage(call, "relationships");
                const authorization = call.requireAuthorization();
                const relationships = await authorization.authorizer.relationships(
                    authorization.snapshot,
                    call.reference(),
                    authorization.access,
                    {
                        ...(listing.after === undefined ? {} : { after: listing.after }),
                        limit: listing.limit + 1,
                    },
                );

                return listing.result(relationships, (relationship) => relationship.id);
            },
        ),
        grant: sharingMethod(
            "grant",
            null,
            { method: "POST", path: "/{id}/relationships" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...GrantShape }),
            Relationship.schema,
            async (call) => {
                // grant only on an object outside the trash
                await requirePresent(call, "live");

                return call.requireAuthorization().grant({
                    ...GrantInput.parse(call.input),
                    object: call.reference(),
                });
            },
            (step) => {
                // revoke the granted relationship
                const granted = Call.resultId(step.result);
                const call =
                    granted === undefined
                        ? undefined
                        : Step.call(step, "revoke", {
                              ...Step.target(step),
                              relationshipId: granted,
                          });

                return call === undefined ? undefined : [call];
            },
        ),
        revoke: sharingMethod(
            "revoke",
            null,
            { method: "DELETE", path: "/{id}/relationships/{relationshipId}" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...RelationshipShape }),
            Empty,
            async (call) => {
                // revoke on any object that exists, in the trash too
                await requirePresent(call, "stored");
                await call
                    .requireAuthorization()
                    .revoke(
                        call.reference(),
                        schema.object(RelationshipShape).parse(call.input).relationshipId,
                    );

                return {};
            },
        ),
        proposals: sharingMethod(
            "proposals",
            grant,
            { method: "GET", path: "/{id}/proposals" },
            (shapes) => shapes.target.extend(PageShape),
            page(Proposal.schema),
            async (call) => {
                // page the object's proposals by identifier
                const listing = sharingPage(call, "proposals");
                const proposals = await call.requireAuthorization().proposals(
                    { object: call.reference() },
                    {
                        ...(listing.after === undefined ? {} : { after: listing.after }),
                        limit: listing.limit + 1,
                    },
                );

                return listing.result(proposals, (proposal) => proposal.id);
            },
        ),
        propose: sharingMethod(
            "propose",
            null,
            { method: "POST", path: "/{id}/proposals" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...ProposeShape }),
            Proposal.schema,
            async (call) => {
                // propose the relationship on an object outside the trash
                // TODO #Incomplete: send an invitation notification when the proposal names a contact
                await requirePresent(call, "live");
                const { relationship, ...proposal } = ProposeInput.parse(call.input);

                return call.requireAuthorization().propose({
                    ...proposal,
                    relationship: { ...relationship, object: call.reference() },
                });
            },
        ),
        accept: sharingMethod(
            "accept",
            null,
            { method: "POST", path: "/{id}/proposals/{proposalId}/accept" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...ProposalShape }),
            Relationship.schema,
            async (call) => {
                // accept only on an object outside the trash
                await requirePresent(call, "live");

                return call
                    .requireAuthorization()
                    .accept(
                        call.reference(),
                        schema.object(ProposalShape).parse(call.input).proposalId,
                    );
            },
        ),
        explain: defineMethod<MethodConfiguration>({
            kind: "explain",
            permission: null,
            mutates: false,
            target: false,
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "GET", path: "/{id}/access" },
                input: shapes.target.extend(ExplainShape),
                output: Explanation,
            }),
            handler: explain,
        }),
        decline: sharingMethod(
            "decline",
            null,
            { method: "DELETE", path: "/{id}/proposals/{proposalId}" },
            (shapes) => shapes.target.extend({ ...shapes.replay, ...ProposalShape }),
            Empty,
            async (call) => {
                // decline on any object that exists, in the trash too
                await requirePresent(call, "stored");
                await call
                    .requireAuthorization()
                    .decline(
                        call.reference(),
                        schema.object(ProposalShape).parse(call.input).proposalId,
                    );

                return {};
            },
        ),
    }),
};

/** Declare a sharing method. */
function sharingMethod(
    kind: MethodKind,
    required: string | null,
    route: Route,
    input: (schema: ObjectSchema) => schema.JsonObject,
    output: MethodProcedure["output"],
    handler: (call: Call) => Promise<unknown>,
    inverse?: (step: Step) => readonly sync.Call[] | undefined,
): Method {
    return defineMethod<MethodConfiguration>({
        kind,
        permission: required,
        mutates: required === null,
        target: required !== null,
        result: "value",
        isPredicted: false,
        procedure: (_name, shapes) => ({ route, input: input(shapes), output }),
        handler,
        ...(inverse === undefined ? {} : { inverse }),
    });
}

/** Explain a subject's permission on an object. */
async function explain(call: Call): Promise<Explanation> {
    // resolve access in the governing scope
    const { authorizer, snapshot } = call.requireAuthorization();
    const { permission: asked, subject } = schema.object(ExplainShape).parse(call.input);
    const object = call.object;
    const target = call.reference();
    const scope = authorizer.governingScope(target);
    const access = await call.requireAuthorization().in(scope);
    const explained = () =>
        asked === undefined ? requireReading(object) : object.permission(asked);

    // explain another subject's access only to grantors
    if (subject !== undefined) {
        const grant = object.policy.definition.grantedBy;
        const granting =
            grant === undefined
                ? undefined
                : await authorizer.check(snapshot, object.permission(grant), target, access);
        if (granting?.isAllowed !== true) {
            throw new ServiceError("FORBIDDEN", {
                message: `explaining access to ${object.name} needs ${grant}`,
            });
        }
        const resolved = await authorizer.resolve(snapshot, scope, {
            subjects: [subject],
            now: call.now,
            attributes: {},
        });

        return authorizer.explain(snapshot, explained(), target, resolved);
    }
    // explain the caller's own access to an object it may read
    else {
        const reading = await authorizer.check(snapshot, requireReading(object), target, access);
        if (!reading.isAllowed) {
            throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${target.id}` });
        }

        return authorizer.explain(snapshot, explained(), target, access);
    }
}

/** Read the permission reading an object of a type needs. */
function requireReading(object: ObjectType) {
    if (object.reading === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `${object.name} has no objects to read` });
    }

    return object.reading;
}

/** Require the named object to exist, outside the trash when live. */
async function requirePresent(call: Call, state: "live" | "stored"): Promise<void> {
    // read the object
    const table = call.object.table;
    const rows: readonly Row[] = await call.database
        .select()
        .from(table)
        .where(
            and(eq(table[TABLE].column("id"), call.requireId()), call.object.inScope(call.scope)),
        );
    const [row] = rows;

    // refuse a missing or trashed object
    if (row === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no ${call.object.name} ${call.id}` });
    } else if (
        state === "live" &&
        row["deletionRequestedAt"] !== undefined &&
        row["deletionRequestedAt"] !== null
    ) {
        throw new ServiceError("CONFLICT", { message: `${call.object.name} is in the trash` });
    }
}

/** Read the page a sharing listing asks for, bound to its object. */
function sharingPage(call: Call, list: string) {
    return new Page(
        call.input,
        [call.object.name, call.scope, call.requireId(), list],
        schema.string(),
    );
}

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
    proposals: Procedure<
        schema.Object<TargetShape<Object> & typeof PageShape>,
        ReturnType<typeof page<typeof Proposal.schema>>
    >;
    propose: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof ProposeShape>,
        typeof Proposal.schema
    >;
    accept: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof ProposalShape>,
        typeof Relationship.schema
    >;
    decline: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & typeof ProposalShape>,
        schema.Object<{}>
    >;
};
