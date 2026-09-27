import {
    jsonElements,
    sql,
    Statement,
    type DatabaseConnection,
    type SQL,
    type SQLWrapper,
} from "@destack/db";
import type { PackageId } from "@destack/package";
import { Replica } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import { permissionKey, type ObjectReference, type PermissionReference } from "../policy/policy.ts";
import { subjectKey, type Subject, type SubjectType } from "../policy/subject.ts";
import { ACCESS_PACKAGE_ID } from "../policy/principal.ts";
import * as principal from "../policy/principal.ts";
import {
    delegationChain,
    isElevated,
    ELEVATION_MILLISECONDS,
    type AccessContext,
} from "../context/context.ts";
import { Restriction } from "../context/restriction.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { Role } from "../role/role.ts";
import type { DefinedRole, RoleGrant } from "../role/closure.ts";
import { COPY_NAME } from "../replica/replica.ts";
import { Authority } from "./authority.ts";
import { GrantCondition, type ConditionContext } from "./condition.ts";
import type { Gate } from "./decision.ts";
import { Authorizer, type ScopeLink } from "./authorizer.ts";
import { column, from, type FieldRelation, TableMapping } from "./mapping.ts";

/** A caller's access in one scope: its authorities, the scope chain, the roles along it, and the time it holds for. */
export class Access {
    /** The scope every protected row of a query belongs to. */
    readonly scope: string;
    /** The caller. */
    readonly context: AccessContext;
    /** The represented subject, then each delegate through what it was lent, every one of which must be admitted. */
    readonly authorities: readonly Authority[];
    /** The objects of the scope and every scope enclosing it, nearest first. */
    readonly scopes: readonly ObjectReference[];
    /** What each role defined along the scope chain grants, itself or through the roles it includes. */
    readonly grants: ReadonlyMap<string, RoleGrant>;
    /** Whether the scope or one enclosing it is suspended, withholding every permission but administration. */
    readonly isSuspended: boolean;
    /** The next moment time alone changes the caller's subject sets, roles or elevation, absent when it never does. */
    readonly until: number | undefined;
    /** The roles along the scope chain that grant each permission, by permission key. */
    readonly #roles: ReadonlyMap<string, readonly string[]>;
    /** The roles along the scope chain that grant every permission but reserved ones, as owners' do. */
    readonly #universal: readonly string[];
    /** The authorizer whose reserved, elevated and administration permissions apply. */
    readonly #authorizer: Authorizer;

    /** Assemble a resolved caller from what its resolution read. */
    constructor(
        authorizer: Authorizer,
        resolved: {
            readonly scope: string;
            readonly context: AccessContext;
            readonly authorities: readonly Authority[];
            readonly links: readonly ScopeLink[];
            readonly grants: ReadonlyMap<string, RoleGrant>;
            readonly until: number | undefined;
        },
    ) {
        // retain what the resolution read
        this.#authorizer = authorizer;
        this.scope = resolved.scope;
        this.context = resolved.context;
        this.authorities = resolved.authorities;
        this.scopes = resolved.links.map((link) => link.object);
        this.grants = resolved.grants;
        this.isSuspended = resolved.links.some((link) => link.isSuspended);
        this.until = resolved.until;

        // index the roles by the permissions they grant, apart from the roles granting everything
        const roles = new Map<string, string[]>();
        const universal: string[] = [];
        for (const [role, grant] of resolved.grants) {
            // list a role granting everything apart
            if (grant.isUniversal) {
                universal.push(role);
            }
            // index any other role under each permission it grants
            else {
                for (const permission of grant.permissions) {
                    const key = permissionKey(permission);
                    roles.set(key, [...(roles.get(key) ?? []), role]);
                }
            }
        }
        this.#roles = roles;
        this.#universal = universal;
    }

    /**
     * Resolve a caller in a scope: its subject sets, the scope chain and the roles along it, in small indexed queries issued together.
     *
     * It refuses to decide on a copy of the chain's access whose home stayed silent past the authorizer's lag.
     */
    static async resolve(
        database: DatabaseConnection,
        scope: string,
        context: AccessContext,
        authorizer: Authorizer,
    ): Promise<Access> {
        // reject invalid request time before reading expiring records
        if (!Number.isFinite(context.now)) {
            throw new AccessError("INVALID_CONTEXT", "request time must be finite");
        }

        // read the chain
        const links = await Authorizer.chain(database, scope);
        const chain = [scope, ...links.map((link) => link.object.id).filter((id) => id !== scope)];

        // read the roles alongside the caller's and every lent delegate's subject sets, refusing copies whose home went silent
        const lent = delegationChain(context).filter((link) => link.authority === "lent");
        const [roles, , represented, delegates] = await Promise.all([
            readRoles(database, chain, context),
            requireConfirmed(database, chain, authorizer.lag),
            Access.expand(database, context.subjects, context, authorizer),
            Promise.all(
                lent.map(async ({ delegate, delegator }) => ({
                    expanded: await Access.expand(database, [delegate], context, authorizer),
                    delegator,
                })),
            ),
        ]);

        // take the earliest moment a subject set, an inclusion or the elevation changes by time
        const assurance = context.assurance;
        const elevation =
            assurance !== undefined && isElevated(context)
                ? assurance.authenticatedAt + ELEVATION_MILLISECONDS
                : undefined;
        const until = earliest([
            roles.until,
            represented.until,
            ...delegates.map((delegate) => delegate.expanded.until),
            elevation,
        ]);

        return new Access(authorizer, {
            scope,
            context,
            authorities: [
                new Authority(represented.subjects),
                ...delegates.map(
                    (delegate) => new Authority(delegate.expanded.subjects, delegate.delegator),
                ),
            ],
            links,
            grants: roles.grants,
            until,
        });
    }

    /** Build the read of the sets a JSON list of subjects belongs to through current relationships, of some set types or every one. */
    static memberships(sets: readonly SubjectType[] | undefined): Statement<SetRow> {
        // look the subject up exactly, and through each wildcard, and as a set
        const relationship = accessRelationship;
        const select = (value: (name: string) => SQLWrapper, scope: SQL, id: SQL, relation: SQL) =>
            sql`SELECT ${relationship.packageId} AS "packageId", ${relationship.type} AS "type", ${relationship.objectScope} AS "scope",
                    ${relationship.objectId} AS "id", ${relationship.relation} AS "relation",
                    ${relationship.createdAt} AS "createdAt", ${relationship.expiresAt} AS "expiresAt", ${relationship.maxAge} AS "maxAge"
                FROM ${jsonElements(value("subjects"), "subject")}
                JOIN ${relationship} ON ${relationship.subjectPackageId} = subject.value ->> 0
                    AND ${relationship.subjectType} = subject.value ->> 1
                    AND ${relationship.subjectScope} = ${scope}
                    AND ${relationship.subjectId} = ${id}
                    AND ${relation}
                WHERE ${sets === undefined ? sql`${relationship.relation} IS NOT NULL` : isSet(sets)}
                    AND ${GrantCondition.where(relationship, GrantCondition.parameters(value))}`;
        const plain = sql`subject.value ->> 4 IS NULL AND ${relationship.subjectRelation} IS NULL`;

        return new Statement<SetRow>((value) =>
            sql.join(
                [
                    select(value, sql`subject.value ->> 2`, sql`subject.value ->> 3`, plain),
                    select(value, sql`subject.value ->> 2`, sql`'*'`, plain),
                    select(value, sql`'*'`, sql`subject.value ->> 3`, plain),
                    select(value, sql`'*'`, sql`'*'`, plain),
                    select(
                        value,
                        sql`subject.value ->> 2`,
                        sql`subject.value ->> 3`,
                        sql`${relationship.subjectRelation} = subject.value ->> 4`,
                    ),
                ],
                sql` UNION ALL `,
            ),
        );
    }

    /** Build the read of the sets a field makes a JSON list of subjects members of: the rows holding them. */
    static fieldSets(
        entry: Pick<FieldRelation, "mapping" | "field">,
    ): Statement<Pick<SetRow, "packageId" | "type" | "scope" | "id">> {
        // match the held subject, in the row's scope unless the field names its subjects' scope
        const table = entry.mapping.table;
        const held = column(table, entry.field.column);
        const scope = TableMapping.scopeColumn(table, entry.mapping);
        const definition = entry.mapping.policy.definition;

        return new Statement(
            (
                value,
            ) => sql`SELECT ${definition.packageId} AS "packageId", ${definition.name} AS "type",
                    ${scope} AS "scope", ${column(table, entry.mapping.id)} AS "id"
                FROM ${jsonElements(value("subjects"), "subject")}
                JOIN ${from(table)} ON ${held} = subject.value ->> 3
                    ${entry.field.scope === undefined ? sql`AND ${scope} = subject.value ->> 2` : sql``}`,
        );
    }

    /** Add every subject set some subjects belong to through current relationships and fields, breadth first, with the next moment time changes them. */
    static async expand(
        database: DatabaseConnection,
        subjects: readonly Subject[],
        context: ConditionContext,
        authorizer?: Pick<Authorizer, "fields" | "sets">,
    ): Promise<{ readonly subjects: Subject[]; readonly until: number | undefined }> {
        // seed with identities and verified subject sets
        const fields = authorizer?.fields ?? [];
        const expanded = [...subjects];
        const known = new Set(expanded.map(subjectKey));
        const bindings = GrantCondition.bindings(context);
        const boundaries: (number | undefined)[] = [];
        let frontier = [...expanded];
        while (frontier.length > 0) {
            // read the sets relationships and fields make the frontier subjects members of, together
            const tuples = (list: readonly Subject[]) => JSON.stringify(subjectTuples(list));
            const [related, ...held] = await Promise.all([
                (authorizer?.sets ?? EVERY_MEMBERSHIP).all(database, {
                    subjects: tuples(frontier),
                    ...bindings,
                }),
                ...fields.map(async (entry) => {
                    const holders = frontier.filter((subject) => isHeldBy(entry, subject));
                    const rows =
                        holders.length === 0
                            ? []
                            : await entry.sets.all(database, { subjects: tuples(holders) });

                    return rows.map((row) => ({ ...row, relation: entry.relation }));
                }),
            ]);

            // note when each relationship making a membership next changes by time
            boundaries.push(...related.map((row) => GrantCondition.boundary(row, context)));

            // continue from the sets not seen before
            frontier = [];
            for (const row of [...related, ...held.flat()]) {
                const subject: Subject = {
                    packageId: row.packageId as PackageId,
                    type: String(row.type),
                    scope: String(row.scope),
                    id: String(row.id),
                    relation: String(row.relation),
                };
                const key = subjectKey(subject);
                if (!known.has(key)) {
                    known.add(key);
                    expanded.push(subject);
                    frontier.push(subject);
                }
            }
        }

        return { subjects: expanded, until: earliest(boundaries) };
    }

    /** List the roles along the scope chain granting a permission, which reserved permissions have none of. */
    granting(permission: PermissionReference): readonly string[] {
        const key = permissionKey(permission);

        return this.#authorizer.reserved.has(key)
            ? []
            : [...(this.#roles.get(key) ?? []), ...this.#universal];
    }

    /** List the roles along the scope chain granting every permission but reserved ones, as owners' do. */
    universal(): readonly string[] {
        return this.#universal;
    }

    /** Read the gate the request fails for a permission before any grant: its credential, its elevation, or the scope's suspension. */
    gate(permission: PermissionReference, id?: string): Gate | undefined {
        // require the credential to allow the permission, on the object among those it selects
        const key = permissionKey(permission);
        if (
            !Restriction.allows(
                permission,
                { scope: this.scope, ...(id === undefined ? {} : { id }) },
                this.context,
            )
        ) {
            return "restricted";
        }
        // require elevation for elevated permissions
        else if (this.#authorizer.elevated.has(key) && !isElevated(this.context)) {
            return "elevation";
        }
        // require an active scope for all but administration
        else if (this.isSuspended && !this.#authorizer.administration.has(key)) {
            return "suspended";
        }

        return undefined;
    }

    /** Read the scope's own object when a mapping holds objects of its type, which lives in the scope containing it. */
    own(mapping: TableMapping): ObjectReference | undefined {
        const own = this.scopes[0];
        const definition = mapping.policy.definition;

        return own !== undefined &&
            own.packageId === definition.packageId &&
            own.type === definition.name
            ? own
            : undefined;
    }
}

/** Refuse the copies of a chain's access whose home stayed silent past the lag, which may keep granting what their home revoked. */
async function requireConfirmed(
    database: DatabaseConnection,
    chain: readonly string[],
    lag: number,
): Promise<void> {
    const now = Date.now();
    for (const [scope, origin] of await Replica.origins(database, COPY_NAME, chain)) {
        if (now - origin.confirmedAt > lag) {
            throw new AccessError(
                "STALE",
                `access of ${scope} was last confirmed by its home ${now - origin.confirmedAt} ms ago`,
            );
        }
    }
}

/** Read the roles a chain of scopes defines with their permissions and current inclusions, closed over the inclusions. */
async function readRoles(
    database: DatabaseConnection,
    chain: readonly string[],
    context: ConditionContext,
): Promise<{ readonly grants: Map<string, RoleGrant>; readonly until: number | undefined }> {
    // read the roles of every scope of the chain
    const rows = await ROLES.all(database, { scopes: JSON.stringify(chain.map((id) => [id])) });

    // read the current inclusions among those roles
    const byRole = Map.groupBy(rows, (row) => row.id);
    const ids = [...byRole.keys()];
    const includes =
        ids.length === 0
            ? []
            : await INCLUSIONS.all(database, {
                  roles: JSON.stringify(ids.map((id) => [byRole.get(id)![0]!.scope, id])),
                  ...GrantCondition.bindings(context),
              });

    // close the roles over their inclusions
    const roles: DefinedRole[] = ids.map((id) => {
        const granted = byRole.get(id)!;

        return {
            id,
            isUniversal: granted[0]!.isUniversal === true || Number(granted[0]!.isUniversal) === 1,
            permissions: granted.flatMap((row) =>
                row.packageId === null
                    ? []
                    : [{ packageId: row.packageId as PackageId, type: row.type!, name: row.name! }],
            ),
        };
    });

    return {
        grants: Role.close(roles, includes),
        until: earliest(includes.map((include) => GrantCondition.boundary(include, context))),
    };
}

/** Take the earliest of some moments, absent when none is present. */
export function earliest(moments: readonly (number | undefined)[]): number | undefined {
    const present = moments.filter((moment): moment is number => moment !== undefined);

    return present.length === 0 ? undefined : Math.min(...present);
}

/** Write subjects as tuples of package, type, scope, identifier and relation, null for a plain subject. */
function subjectTuples(subjects: readonly Subject[]): unknown[][] {
    return subjects.map((subject) => [
        subject.packageId,
        subject.type,
        subject.scope,
        subject.id,
        subject.relation ?? null,
    ]);
}

/** Match a relationship relating an object's members through a relation some relation accepts as a subject set. */
function isSet(sets: readonly SubjectType[]): SQL {
    const relationship = accessRelationship;

    return sets.length === 0
        ? sql`false`
        : sql`(${sql.join(
              sets.map(
                  (set) =>
                      sql`(${relationship.packageId} = ${set.packageId} AND ${relationship.type} = ${set.type} AND ${relationship.relation} = ${set.relation})`,
              ),
              sql` OR `,
          )})`;
}

/** Decide whether a field may hold a subject: one of the subject type it holds, in its scope. */
function isHeldBy(entry: FieldRelation, subject: Subject): boolean {
    return (
        subject.packageId === entry.subject.packageId &&
        subject.type === entry.subject.type &&
        subject.relation === entry.subject.relation &&
        (entry.field.scope === undefined || entry.field.scope === subject.scope)
    );
}

/** A role a scope list defines, one row per permission it grants itself, or one without any. */
type RoleRow = {
    /** The role. */
    readonly id: string;
    /** The scope defining the role. */
    readonly scope: string;
    /** Whether the role grants everything, as the driver returns a boolean. */
    readonly isUniversal: boolean | number;
    /** The permission's package, null for a role granting none itself. */
    readonly packageId: string | null;
    /** The permission's type. */
    readonly type: string | null;
    /** The permission's name. */
    readonly name: string | null;
};

/** The roles a JSON list of scopes defines, with the permissions each grants itself. */
const ROLES = new Statement<RoleRow>(
    (
        value,
    ) => sql`SELECT ${accessRole.id} AS "id", ${accessRole.scope} AS "scope", ${accessRole.isUniversal} AS "isUniversal",
            ${accessRolePermission.packageId} AS "packageId", ${accessRolePermission.type} AS "type",
            ${accessRolePermission.name} AS "name"
        FROM ${jsonElements(value("scopes"), "listed_scope")}
        JOIN ${accessRole} ON ${accessRole.scope} = listed_scope.value ->> 0
        LEFT JOIN ${accessRolePermission} ON ${accessRolePermission.roleId} = ${accessRole.id}`,
);

/** The current inclusions among a JSON list of roles, each as scope and identifier, under a request's conditions, with their times. */
const INCLUSIONS = new Statement<{
    role: string;
    included: string;
    createdAt: number | string;
    expiresAt: number | string | null;
    maxAge: number | string | null;
}>(
    (
        value,
    ) => sql`SELECT ${accessRelationship.objectId} AS "role", ${accessRelationship.subjectId} AS "included",
            ${accessRelationship.createdAt} AS "createdAt", ${accessRelationship.expiresAt} AS "expiresAt",
            ${accessRelationship.maxAge} AS "maxAge"
        FROM ${jsonElements(value("roles"), "listed_role")}
        JOIN ${accessRelationship} ON ${accessRelationship.objectScope} = listed_role.value ->> 0
            AND ${accessRelationship.packageId} = ${ACCESS_PACKAGE_ID}
            AND ${accessRelationship.type} = ${principal.role.name}
            AND ${accessRelationship.objectId} = listed_role.value ->> 1
        WHERE ${accessRelationship.relation} = 'includes'
            AND ${accessRelationship.subjectPackageId} = ${ACCESS_PACKAGE_ID}
            AND ${accessRelationship.subjectType} = ${principal.role.name}
            AND ${accessRelationship.subjectRelation} IS NULL
            AND ${GrantCondition.where(accessRelationship, GrantCondition.parameters(value))}`,
);

/** A subject set a subject belongs to, with the times of the relationship making it a member. */
export type SetRow = {
    /** The set object's package. */
    readonly packageId: string;
    /** The set object's type. */
    readonly type: string;
    /** The set object's scope. */
    readonly scope: string;
    /** The set object's identifier. */
    readonly id: string;
    /** The set's relation. */
    readonly relation: string;
    /** When the membership started. */
    readonly createdAt: number | string;
    /** When the membership expires, null when it never does. */
    readonly expiresAt: number | string | null;
    /** The longest time since authentication the membership allows, null when it asks none. */
    readonly maxAge: number | string | null;
};

/** Read every set relation's memberships, for callers expanding outside an authorizer. */
const EVERY_MEMBERSHIP = Access.memberships(undefined);
