import type { DatabaseConnection } from "@destack/db";
import type { Snapshot } from "@destack/db/log";
import type { PackageId } from "@destack/package";
import { Replica } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import { permissionKey, type ObjectReference, type PermissionReference } from "../policy/policy.ts";
import { subjectKey, type Subject, type SubjectType } from "../policy/subject.ts";
import { ACCESS_PACKAGE_ID } from "../policy/principal.ts";
import * as principal from "../policy/principal.ts";
import { delegationChain, type AccessContext } from "../context/context.ts";
import { Elevation } from "../context/elevation.ts";
import { Restriction } from "../context/restriction.ts";
import { Relationship } from "../relationship/relationship.ts";
import type { RelationshipRow } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { Role } from "../role/role.ts";
import type { DefinedRole, RoleGrant } from "../role/closure.ts";
import { COPY_NAME } from "../replica/replica.ts";
import { Authority } from "./authority.ts";
import { GrantCondition, type ConditionContext } from "./condition.ts";
import type { Gate } from "./decision.ts";
import { Authorizer } from "./authorizer.ts";
import type { ScopeLink } from "../scope/scope.ts";
import { type FieldRelation, TableMapping } from "./mapping.ts";
import { Scope } from "../scope/scope.ts";

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
    /** The fenced scope nearest in the chain and the holder it moves to. */
    readonly moved: { readonly scope: string; readonly holder: string } | undefined;
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
        const fenced = resolved.links.find((link) => link.movedTo !== undefined);
        this.moved =
            fenced === undefined ? undefined : { scope: fenced.object.id, holder: fenced.movedTo! };
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
     * Resolve a caller in a scope as a snapshot shows its access: its subject sets, the scope chain and the roles along it, in small indexed reads issued together.
     *
     * It refuses to decide on a copy of the chain's access whose home stayed silent past the authorizer's lag.
     * A caller that read the chain in the same snapshot passes its links as `known`.
     */
    static async resolve(
        snapshot: Snapshot,
        scope: string,
        context: AccessContext,
        authorizer: Authorizer,
        known?: readonly ScopeLink[],
    ): Promise<Access> {
        // reject invalid request time before reading expiring records
        if (!Number.isFinite(context.now)) {
            throw new AccessError("INVALID_CONTEXT", "request time must be finite");
        }

        // read the chain, unless the caller read it in the same snapshot already
        const links = known ?? (await Scope.chain(snapshot, scope));
        const chain = [scope, ...links.map((link) => link.object.id).filter((id) => id !== scope)];

        // read the roles alongside the caller's and every lent delegate's subject sets, refusing copies whose home went silent
        const lent = delegationChain(context).filter((link) => link.authority === "lent");
        const [roles, , represented, delegates] = await Promise.all([
            readRoles(snapshot, chain, context),
            snapshot.position === undefined
                ? requireConfirmed(snapshot.database, chain, authorizer.lag)
                : undefined,
            Access.expand(snapshot, context.subjects, context, authorizer),
            Promise.all(
                lent.map(async ({ delegate, delegator }) => ({
                    expanded: await Access.expand(snapshot, [delegate], context, authorizer),
                    delegator,
                })),
            ),
        ]);

        // take the earliest moment a subject set, an inclusion or an elevation the caller holds changes by time
        const until = earliest([
            roles.until,
            represented.until,
            ...delegates.map((delegate) => delegate.expanded.until),
            ...[...authorizer.elevated.values()].map((elevation) =>
                Elevation.until(elevation, context),
            ),
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

    /** Add every subject set some subjects belong to through current relationships and fields, breadth first, with the next moment time changes them. */
    static async expand(
        snapshot: Snapshot,
        subjects: readonly Subject[],
        context: ConditionContext,
        authorizer?: Pick<Authorizer, "fields" | "memberships">,
    ): Promise<{
        readonly subjects: Subject[];
        readonly until: number | undefined;
    }> {
        // seed with identities and verified subject sets
        const fields = authorizer?.fields ?? [];
        const expanded = [...subjects];
        const known = new Set(expanded.map(subjectKey));
        const boundaries: (number | undefined)[] = [];
        let frontier = [...expanded];
        while (frontier.length > 0) {
            // read the sets relationships and fields make the frontier subjects members of, together
            const [related, ...held] = await Promise.all([
                memberships(snapshot, frontier, context, authorizer?.memberships),
                ...fields.map((entry) => fieldSets(snapshot, entry, frontier)),
            ]);

            // note when each relationship making a membership next changes by time
            boundaries.push(...related.map((row) => GrantCondition.boundary(row, context)));

            // continue from the sets not seen before
            frontier = [];
            const found = [
                ...related.map((row) => ({
                    packageId: row.packageId,
                    type: row.type,
                    scope: row.objectScope,
                    id: row.objectId,
                    relation: row.relation!,
                })),
                ...held.flat(),
            ];
            for (const subject of found) {
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

    /** Read the gate a request fails on a row before any grant: its credential, elevation or suspension. */
    gate(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
    ): Gate | undefined {
        return this.admits(permission, String(row[mapping.id]), { mapping, row })
            ? this.blocked(permission)
            : "restricted";
    }

    /**
     * Decide whether the credential allows a permission on an object, directly or through its source.
     *
     * A derivation through a relation needs the object's row, which names the related object.
     */
    admits(
        permission: PermissionReference,
        id: string,
        stored?: {
            readonly mapping: TableMapping;
            readonly row: Readonly<Record<string, unknown>>;
        },
    ): boolean {
        // allow a permission the credential names
        if (Restriction.allows(permission, { scope: this.scope, id }, this.context)) {
            return true;
        }

        // allow a derivation as its source is allowed: on the same object, or on the related one the row names
        const source = this.#authorizer.source(permission);
        if (source === undefined) {
            return false;
        } else if (source.relation === undefined) {
            return this.admits({ ...permission, name: source.permission }, id, stored);
        } else if (stored === undefined) {
            return false;
        }
        const related = this.#authorizer.related(stored.mapping, source.relation, stored.row);

        return (
            related !== undefined &&
            related.scope === this.scope &&
            this.admits(
                { packageId: related.packageId, type: related.type, name: source.permission },
                related.id,
            )
        );
    }

    /** Read the gate a request fails on any object: its elevation or the scope's suspension. */
    blocked(permission: PermissionReference): Gate | undefined {
        const key = permissionKey(permission);

        // require the authentication an elevated permission asks for
        if (!this.elevates(permission)) {
            return "elevation";
        }
        // require an active scope for all but administration
        else if (this.isSuspended && !this.#authorizer.administration.has(key)) {
            return "suspended";
        }

        return undefined;
    }

    /** Report whether the caller authenticated as a permission's elevation asks, as every caller does for a permission without one. */
    elevates(permission: PermissionReference): boolean {
        const elevation = this.#authorizer.elevated.get(permissionKey(permission));

        return elevation === undefined || Elevation.admits(elevation, this.context);
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
    snapshot: Snapshot,
    chain: readonly string[],
    context: ConditionContext,
): Promise<{
    readonly grants: Map<string, RoleGrant>;
    readonly until: number | undefined;
}> {
    // read the roles of every scope of the chain, their permissions, and the relationships on them
    const rows = await snapshot.select(
        accessRole,
        ["scope"],
        chain.map((id) => [id]),
    );
    const ids = rows.map((row) => [String(row.id)]);
    const objects = rows.map((row) => principal.role.reference(String(row.scope), String(row.id)));
    const [permissions, relationships] = await Promise.all([
        snapshot.select(accessRolePermission, ["roleId"], ids),
        Relationship.readByObject(snapshot, objects),
    ]);

    // keep the current inclusions among those roles
    const byRole = Map.groupBy(permissions, (row) => String(row.roleId));
    const includes = relationships.filter(
        (row) =>
            row.relation === "includes" &&
            row.subjectPackageId === ACCESS_PACKAGE_ID &&
            row.subjectType === principal.role.name &&
            row.subjectRelation === null &&
            new GrantCondition(row).failure(context, undefined, undefined) === undefined,
    );

    // close the roles over their inclusions
    const roles: DefinedRole[] = rows.map((row) => ({
        id: String(row.id),
        isUniversal: row.isUniversal === true,
        permissions: (byRole.get(String(row.id)) ?? []).map((granted) => ({
            packageId: granted.packageId as PackageId,
            type: String(granted.type),
            name: String(granted.name),
        })),
    }));

    return {
        grants: Role.close(
            roles,
            includes.map((include) => ({
                role: include.objectId,
                included: include.subjectId,
            })),
        ),
        until: earliest(includes.map((include) => GrantCondition.boundary(include, context))),
    };
}

/** Read the relationships making subjects members of sets of some types, or of every type, under a request's conditions. */
async function memberships(
    snapshot: Snapshot,
    subjects: readonly Subject[],
    context: ConditionContext,
    sets: readonly SubjectType[] | undefined,
): Promise<RelationshipRow[]> {
    const rows = await Relationship.readBySubject(snapshot, subjects);

    return rows.filter(
        (row) =>
            row.relation !== null &&
            (sets === undefined ||
                sets.some(
                    (set) =>
                        set.packageId === row.packageId &&
                        set.type === row.type &&
                        set.relation === row.relation,
                )) &&
            new GrantCondition(row).failure(context, undefined, undefined) === undefined,
    );
}

/** Read the sets a field makes some subjects members of: the rows holding them, in the row's scope unless the field names its subjects' scope. */
async function fieldSets(
    snapshot: Snapshot,
    entry: FieldRelation,
    subjects: readonly Subject[],
): Promise<Subject[]> {
    // read the rows holding the subjects the field may hold
    const holders = subjects.filter((subject) => isHeldBy(entry, subject));
    if (holders.length === 0) {
        return [];
    }
    const { mapping, field } = entry;
    const rows = await snapshot.select(
        mapping.table,
        [field.column],
        [...new Set(holders.map((subject) => subject.id))].map((id) => [id]),
    );

    // keep the rows holding a subject of their scope, or of any scope the field names
    const definition = mapping.policy.definition;
    const held = new Set(holders.map(subjectKey));

    return rows.flatMap((row) => {
        const scope = TableMapping.scope(mapping, row);
        const subject = {
            ...entry.subject,
            scope: field.scope ?? scope,
            id: String(row[field.column]),
        };

        return held.has(subjectKey(subject))
            ? [
                  {
                      packageId: definition.packageId,
                      type: definition.name,
                      scope,
                      id: String(row[mapping.id]),
                      relation: entry.relation,
                  },
              ]
            : [];
    });
}

/** Take the earliest of some moments, absent when none is present. */
export function earliest(moments: readonly (number | undefined)[]): number | undefined {
    const present = moments.filter((moment): moment is number => moment !== undefined);

    return present.length === 0 ? undefined : Math.min(...present);
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
