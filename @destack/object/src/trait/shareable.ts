import {
    AccessContext,
    type AccessExpression,
    anyone,
    Caller,
    Explanation,
    permission,
    principal,
    relation,
    Relationship,
    through,
    union,
} from "@destack/access";
import { AccessName, Subject } from "@destack/sync";
import type * as sync from "@destack/sync";
import { Digest, Instant, schema } from "@destack/schema";
import { t } from "@destack/locale";
import { CallError } from "../error/call.ts";
import { ObjectError } from "../error/error.ts";
import { field } from "../field/field.ts";
import { MethodContext } from "../method/context.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";
import type { InputObject } from "../method/input.ts";
import { defineMethod, Method, type MethodKind } from "../method/method.ts";
import { Page } from "../method/page.ts";
import {
    Empty,
    ListPage,
    type MethodProcedure,
    type ObjectSchema,
    PageFields,
    type Procedure,
    type ReplayFields,
    type Route,
    type TargetFields,
} from "../method/procedure.ts";
import { Step } from "../method/step.ts";
import { ObjectPolicy } from "../object/access.ts";
import {
    ObjectPermissions,
    type ObjectDefinition,
    type ObjectRelationInput,
    type ObjectType,
} from "../object/object.ts";
import type { TraitPermission, Erasure, TraitBehavior } from "./trait.ts";
import { callableTrait } from "./trait.ts";
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
    role: schema.identifier("role").exactOptional(),
    ...SubjectFields,
};

/** A relationship sharing methods grant: through exactly one declared relation or role. */
export const GrantInput = schema.union([
    schema.object({ ...SubjectFields, relation: AccessName }),
    schema.object({ ...SubjectFields, role: schema.identifier("role") }),
]);

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
          readonly explain: Method<{ kind: "explain"; permission: string; mutates: false }>;
      }
    : {};

/** A permission the roles grant: membership, read, comment, edit or manage. */
export type RolePermission = (typeof ROLE_PERMISSIONS)[number];

/** Sharing through the roles: owners, editors, commenters and viewers, shared by `manage`. */
export interface RolesDefinition {
    /** Whether anyone may have the roles below owner, as Drive's general access and links allow. */
    readonly isPublic?: true;
}

/** How callers share objects: through roles, or separate relations by permission, and who sees. */
export type ShareableDefinition<Permission extends string = string> = (
    | TraitPermission<Permission>
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
export type SharingGateOf<Sharing> = [Sharing] extends [TraitPermission<infer Permission>]
    ? Permission
    : [Sharing] extends [RolesDefinition]
      ? "manage"
      : undefined;

/** The permissions the roles grant when objects are shared through them, none otherwise. */
export type RolePermissionsOf<Sharing> = [Sharing] extends [TraitPermission]
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
                `object '${definition.name}' shares through roles, so it must declare its permissions as expressions`,
            );
        }
        const declared = permissions;

        // refuse the relations the roles define, and an owner field with no subject
        const owner = Object.entries(definition.fields ?? {}).find(
            ([property, candidate]) => candidate.relationAt(property) === "owner",
        )?.[1];
        const taken = [
            ...(owner !== undefined && owner.type !== "subject" ? ["owner"] : []),
            ...ROLES.filter((name) => Object.hasOwn(definition.relations ?? {}, name)),
        ];
        if (taken.length > 0) {
            throw new TypeError(
                `object '${definition.name}' shares through roles and cannot declare '${taken.join(", ")}'`,
            );
        }

        // relate users, installations and group members through the roles
        const subjects = [
            principal.user,
            principal.installation,
            principal.group.members("member"),
        ];

        // add anyone below owner on a public type
        const shared = roles.isPublic === true ? [...subjects, anyone.all()] : subjects;
        const relations: Record<string, ObjectRelationInput> = {
            ...(definition.isScope === true ? { owner: { subjects } } : {}),
            editor: { subjects: shared },
            commenter: { subjects: shared },
            viewer: { subjects: shared },
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

        // widen each by what the definition adds
        const added = (name: RolePermission) =>
            declared[name] === undefined ? [] : [declared[name]];

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
                    typeof scope !== "string" && "roles" in scope && scope.roles !== undefined,
            );
    },
};

/** Objects callers share through relationships and invitations. */
const shareableTrait: TraitBehavior<SharingGate> = {
    key: "shareable",
    storages: ["durable"],
    options: (definition) => {
        // derive no sharing methods for a partitioned scope, shared through its relationships
        const grant = Roles.gate(definition);
        const isPartition =
            definition.partition === true || definition.represents?.definition.partition === true;

        return grant === undefined || isPartition
            ? undefined
            : { grant, read: definition.shareable?.read ?? "read" };
    },
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => Shareable.methods(options),
};

/** Share objects through relationships and invitations. */
export const shareable = callableTrait("shareable", shareableTrait);

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
};

/** The signature of a sharing method, acting on one object by its identifier. */
type SharingConfiguration = {
    kind: MethodKind;
    permission: string;
    mutates: boolean;
    target: "object" | "id";
};

/** The context of a sharing method. */
type SharingContext = MethodContext<SharingConfiguration>;
/** The shareable methods' handlers: sharing an object and paging its relationships. */
export const Shareable = {
    /** Read a grant through exactly one declared relation or role, refusing both or neither. */
    readGrant(input: unknown): schema.Infer<typeof GrantInput> {
        const grant = GrantInput.safeParse(input);
        if (!grant.success) {
            throw ObjectError.of(
                "CALL_INVALID",
                "grant is invalid: it must have exactly one relation or role",
            );
        }

        return grant.data;
    },

    /** Declare an object's sharing methods: relationships, grants, invitations and explanations. */
    methods({ grant, read }: SharingGate) {
        return {
            relationships: Shareable.sharingMethod(
                "relationships",
                { permission: read, mutates: false },
                { method: "GET", path: "/{id}/relationships" },
                (schemas) => schemas.target.extend(PageFields),
                ListPage.schema(Relationship.schema),
                async (call, { authorization }) => {
                    // page the object's relationships by identifier
                    const listing = Shareable.sharingPage(call, "relationships");
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
            grant: Shareable.sharingMethod(
                "grant",
                { permission: grant, mutates: true },
                { method: "POST", path: "/{id}/relationships" },
                (schemas) => schemas.target.extend({ ...schemas.replay, ...GrantFields }),
                Relationship.schema,
                async (call, execution) => {
                    // grant only on an object outside the trash
                    await ObjectPolicy.requireObject(
                        call.reference(),
                        { deleted: "exclude" },
                        execution,
                    );

                    return Relationship.grant(
                        {
                            ...Shareable.readGrant(call.input),
                            object: call.reference(),
                        },
                        execution.authorization,
                    );
                },
                (step) => {
                    // revoke the granted relationship
                    const granted = Method.resultId(step.result);
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
            revoke: Shareable.sharingMethod(
                "revoke",
                { permission: grant, mutates: true },
                { method: "DELETE", path: "/{id}/relationships/{relationshipId}" },
                (schemas) => schemas.target.extend({ ...schemas.replay, ...RelationshipFields }),
                Empty,
                async (call, execution) => {
                    // revoke on any object that exists, in the trash too
                    await ObjectPolicy.requireObject(
                        call.reference(),
                        { deleted: "include" },
                        execution,
                    );
                    await Relationship.revoke(
                        call.reference(),
                        schema.object(RelationshipFields).parse(call.input).relationshipId,
                        execution.authorization,
                    );

                    return {};
                },
            ),
            explain: defineMethod<SharingConfiguration>({
                title: t`Explain`,
                kind: "explain",
                permission: read,
                mutates: false,
                target: "id",
                result: "value",
                isPredicted: false,
                procedure: (_name, schemas) => ({
                    route: { method: "GET", path: "/{id}/access" },
                    input: schemas.target.extend(ExplainFields),
                    output: Explanation,
                }),
                handler: (context, execution) =>
                    Shareable.explain(context, Execution.store(execution, context)),
            }),
        };
    },

    /** Declare a sharing method: a listing loads its target, a change defers to its relation. */
    sharingMethod(
        kind: MethodKind,
        access: { readonly permission: string; readonly mutates: boolean },
        route: Route,
        input: (schema: ObjectSchema) => InputObject,
        output: MethodProcedure["output"],
        handler: (context: SharingContext, execution: StoreExecution) => Promise<unknown>,
        inverse?: (step: Step) => readonly sync.Call[] | undefined,
    ): Method {
        return defineMethod<SharingConfiguration>({
            kind,
            permission: access.permission,
            mutates: access.mutates,
            target: access.mutates ? "id" : "object",
            result: "value",
            isPredicted: false,
            procedure: (_name, schemas) => ({ route, input: input(schemas), output }),
            handler: (context, execution) => handler(context, Execution.store(execution, context)),
            ...(inverse === undefined ? {} : { inverse }),
        });
    },

    /** Explain a subject's permission on an object. */
    async explain(
        context: SharingContext,
        { authorization }: StoreExecution,
    ): Promise<Explanation> {
        // resolve access in the governing scope
        const { authorizer, snapshot } = authorization;
        const { permission: asked, subject } = schema.object(ExplainFields).parse(context.input);
        const object = context.object;
        const target = context.reference();
        const scope = authorizer.governingScope(target);
        const access = await authorization.in(scope);
        const explained = () =>
            asked === undefined ? Shareable.requireReading(object) : object.permission(asked);

        // explain another subject's access only to grantors
        if (subject !== undefined) {
            const grant = object.policy.definition.grantedBy;
            const granting =
                grant === undefined
                    ? undefined
                    : await authorizer.check(snapshot, object.permission(grant), target, access);
            if (granting?.isAllowed !== true) {
                throw ObjectError.of(
                    "EXPLAIN_DENIED",
                    `access explanation of ${object.name} is forbidden: it needs '${grant}'`,
                );
            }
            const resolved = await authorizer.resolve(
                snapshot,
                scope,
                AccessContext.of(Caller.of(subject), context.now),
            );

            return authorizer.explain(snapshot, explained(), target, resolved);
        }
        // explain the caller's access to an object it may read
        else {
            const reading = await authorizer.check(
                snapshot,
                Shareable.requireReading(object),
                target,
                access,
            );
            if (!reading.isAllowed) {
                throw CallError.http("NOT_FOUND", {
                    message: `${object.name} '${target.id}' not found`,
                });
            }

            return authorizer.explain(snapshot, explained(), target, access);
        }
    },

    /** Read the permission reading an object of a type needs. */
    requireReading(object: ObjectType) {
        if (object.reading === undefined) {
            throw ObjectError.of(
                "READING_REQUIRED",
                `object of ${object.name} to read is required`,
            );
        }

        return object.reading;
    },

    /** Read the page a sharing listing asks for, bound to its object. */
    sharingPage(context: SharingContext, list: string) {
        return new Page(
            context.input,
            [context.object.name, context.scope, context.id, list],
            schema.string(),
        );
    },
};
