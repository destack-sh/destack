import type { DatabaseConnection, Select, Snapshot } from "@destack/db";
import { Replica, type ScopeLink, Scope, type ObjectReference, Subject } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../policy/policy.ts";
import { type SubjectType } from "../policy/subject.ts";
import { ACCESS_PACKAGE_ID } from "../policy/principal.ts";
import * as principal from "../policy/principal.ts";
import { AccessContext, Caller, Contact } from "../context/context.ts";
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
import { type FieldRelation, TableMapping } from "./mapping.ts";

/** A caller's access in one scope: its authorities, the scope chain, the roles along it, and the time it is valid for. */
export class Access {
    /** The scope every protected row of a query belongs to. */
    readonly scope: string;
    /** The caller. */
    readonly context: AccessContext;
    /** The represented subject and each delegate with its lent authority, all of which must be admitted. */
    readonly authorities: readonly Authority[];
    /** The objects of the scope and every scope enclosing it, nearest first. */
    readonly scopes: readonly ObjectReference[];
    /** What each role defined along the scope chain grants, itself or through the roles it includes. */
    readonly grants: ReadonlyMap<string, RoleGrant>;
    /** Whether the scope or one enclosing it is suspended, withholding every permission but administration. */
    readonly isSuspended: boolean;
    /** The fenced scope nearest in the chain and the cell it moves to. */
    readonly moved: { readonly scope: string; readonly cell: string } | undefined;
    /** The next moment time changes the caller's subject sets, roles or elevation. */
    readonly until: number | undefined;
    /** The roles along the scope chain that grant each permission, by permission key. */
    readonly #roles: ReadonlyMap<string, readonly string[]>;
    /** The roles along the scope chain that grant every permission but reserved ones, as owners' do. */
    readonly #universal: readonly string[];
    /** The authorizer whose reserved, elevated and administration permissions apply. */
    readonly #authorizer: Authorizer;
    /** The scope and every scope enclosing it, nearest first. */
    readonly #links: readonly ScopeLink[];

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
        this.#links = resolved.links;
        this.scopes = resolved.links.map((link) => link.object);
        this.grants = resolved.grants;
        this.isSuspended = resolved.links.some((link) => link.isSuspended);
        const [moved] = resolved.links.flatMap((link) =>
            link.movedTo === undefined ? [] : [{ scope: link.object.id, cell: link.movedTo }],
        );
        this.moved = moved;
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
                    const key = PermissionReference.key(permission);
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
     * It refuses to decide on a stale copy of the chain's access.
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
        const scopes = links.map((link) => link.object);

        // read the roles and the subject sets of the caller and every lent delegate
        const lent = Caller.delegation(context).filter((link) => link.authority === "lent");
        const [roles, , represented, delegates] = await Promise.all([
            readRoles(snapshot, chain, context),
            snapshot.position === undefined
                ? requireConfirmed(snapshot.database, chain, authorizer.lag)
                : undefined,
            Access.expand(
                snapshot,
                [
                    ...context.subjects,
                    ...Caller.contacts(context).map((contact) => Contact.subject(contact)),
                ],
                context,
                authorizer,
                scopes,
            ),
            Promise.all(
                lent.map(async ({ delegate, delegator }) => ({
                    expanded: await Access.expand(
                        snapshot,
                        [delegate],
                        context,
                        authorizer,
                        scopes,
                    ),
                    delegation: { delegate, delegator },
                })),
            ),
        ]);

        // take the earliest moment a subject set, an inclusion or an elevation the caller has changes by time
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
                    (delegate) => new Authority(delegate.expanded.subjects, delegate.delegation),
                ),
            ],
            links,
            grants: roles.grants,
            until,
        });
    }

    /**
     * Resolve the caller in scopes this one encloses, in reads shared by all of them.
     *
     * The caller's subject sets and the roles along this scope's chain are reused.
     * A scope this one does not enclose is left out.
     */
    async descend(snapshot: Snapshot, scopes: readonly string[]): Promise<Map<string, Access>> {
        // read the scopes' rows and the rows between them and this scope
        const chain = new Set(this.#links.map((link) => link.object.id));
        const rows = new Map<string, Select<typeof Scope.table>>();
        for (let wanted = scopes.filter((id) => !chain.has(id)); wanted.length > 0;) {
            const read = await snapshot.select(
                Scope.table,
                ["scope"],
                wanted.map((id) => [id]),
            );
            for (const row of read) {
                rows.set(row.scope, row);
            }
            wanted = [...new Set(read.map((row) => row.parent))].filter(
                (id) => !rows.has(id) && !chain.has(id),
            );
        }

        // link each scope up to this one, leaving out the scopes it does not enclose
        const below = new Map<string, ScopeLink[]>();
        for (const scope of new Set(scopes)) {
            const links: ScopeLink[] = [];
            let current = rows.get(scope);
            const visited = new Set<string>();
            while (current !== undefined && !visited.has(current.scope)) {
                visited.add(current.scope);
                links.push({
                    object: {
                        packageId: current.packageId,
                        type: current.type,
                        scope: current.parent,
                        id: current.scope,
                    },
                    parent: current.parent,
                    isSuspended: current.suspendedAt !== null,
                    movedTo: current.movedTo ?? undefined,
                });
                if (current.parent === this.scope) {
                    below.set(scope, links);
                    break;
                }
                current = rows.get(current.parent);
            }
        }

        // refuse stale copies of the scopes' access, and find the scopes defining roles
        const ids = [...new Set([...below.values()].flat().map((link) => link.object.id))];
        const [defining] = await Promise.all([
            snapshot.select(
                accessRole,
                ["scope"],
                ids.map((id) => [id]),
            ),
            snapshot.position === undefined
                ? requireConfirmed(snapshot.database, ids, this.#authorizer.lag)
                : undefined,
        ]);
        const defined = new Set(defining.map((row) => row.scope));

        // reuse this scope's roles below it, reading the roles of a chain that defines more
        const resolved = new Map<string, Access>();
        for (const [scope, links] of below) {
            // expand the authorities again for a chain whose scopes take sets from the scopes enclosing them
            const objects = [...links, ...this.#links].map((link) => link.object);
            const expansions = links.some((link) => this.#authorizer.isEnclosed(link.object))
                ? await Promise.all(
                      this.authorities.map(async (authority) => ({
                          authority,
                          expanded: await Access.expand(
                              snapshot,
                              authority.subjects,
                              this.context,
                              this.#authorizer,
                              objects,
                          ),
                      })),
                  )
                : [];
            const authorities =
                expansions.length === 0
                    ? this.authorities
                    : expansions.map(
                          ({ authority, expanded }) =>
                              new Authority(expanded.subjects, authority.delegation),
                      );

            const roles = links.some((link) => defined.has(link.object.id))
                ? await readRoles(
                      snapshot,
                      [...links, ...this.#links].map((link) => link.object.id),
                      this.context,
                  )
                : { grants: this.grants, until: undefined };
            resolved.set(
                scope,
                new Access(this.#authorizer, {
                    scope,
                    context: this.context,
                    authorities,
                    links: [...links, ...this.#links],
                    grants: roles.grants,
                    until: earliest([
                        this.until,
                        roles.until,
                        ...expansions.map(({ expanded }) => expanded.until),
                    ]),
                }),
            );
        }

        return resolved;
    }

    /**
     * Add the subject sets of some subjects breadth first, with the next moment time changes them.
     *
     * A set on a scope reaches the sets of the scopes it encloses along the chain.
     */
    static async expand(
        snapshot: Snapshot,
        subjects: readonly Subject[],
        context: ConditionContext,
        authorizer: Pick<Authorizer, "fields" | "memberships" | "implied" | "enclosed">,
        chain: readonly ObjectReference[],
    ): Promise<{
        readonly subjects: Subject[];
        readonly until: number | undefined;
    }> {
        // admit the identities and verified subject sets, with the sets they reach on the chain
        const expanded = new Map<string, Subject>();
        const boundaries: (number | undefined)[] = [];
        let frontier = Access.#admit(subjects, expanded, authorizer, chain);
        while (frontier.length > 0) {
            // read the sets relationships and fields make the frontier subjects members of, together
            const level = frontier;
            const [related, ...fielded] = await Promise.all([
                memberships(snapshot, level, context, authorizer.memberships),
                ...authorizer.fields.map((entry) => fieldSets(snapshot, entry, level)),
            ]);

            // note when each relationship making a membership next changes by time
            boundaries.push(...related.map((row) => GrantCondition.boundary(row, context)));

            // continue from each relationship's set and the permissions accepted as subject sets it decides
            const found = [
                ...related.flatMap((row) => {
                    // refuse a role binding among the memberships
                    if (row.relation === null) {
                        throw new TypeError(`membership ${row.id} has no relation`);
                    }

                    return [row.relation, ...authorizer.implied(row, row.relation)].map(
                        (relation) => ({
                            packageId: row.packageId,
                            type: row.type,
                            scope: row.objectScope,
                            id: row.objectId,
                            relation,
                        }),
                    );
                }),
                ...fielded.flat(),
            ];
            frontier = Access.#admit(found, expanded, authorizer, chain);
        }

        return { subjects: [...expanded.values()], until: earliest(boundaries) };
    }

    /** Add the subjects not seen before to an expansion, with the sets they reach on the chain's enclosed scopes, returning those added. */
    static #admit(
        found: readonly Subject[],
        expanded: Map<string, Subject>,
        authorizer: Pick<Authorizer, "enclosed">,
        chain: readonly ObjectReference[],
    ): Subject[] {
        // walk the found subjects and the sets they reach, in order
        const added: Subject[] = [];
        const pending = [...found];
        for (const subject of pending) {
            // take each subject not seen before, in order
            const key = Subject.key(subject);
            if (expanded.has(key)) {
                continue;
            }
            expanded.set(key, subject);
            added.push(subject);

            // queue the sets it reaches on the chain's scopes it encloses
            const enclosed =
                subject.relation === undefined
                    ? []
                    : authorizer.enclosed(subject, subject.relation);
            for (const set of enclosed) {
                pending.push(
                    ...chain
                        .filter(
                            (scope) =>
                                scope.packageId === set.packageId &&
                                scope.type === set.type &&
                                scope.scope === subject.id,
                        )
                        .map((scope) => ({ ...scope, relation: set.name })),
                );
            }
        }

        return added;
    }

    /** List the roles along the scope chain that grant a permission. */
    granting(permission: PermissionReference): readonly string[] {
        const key = PermissionReference.key(permission);

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
     * A derivation through a relation needs the object's row to find the related object.
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
        const key = PermissionReference.key(permission);

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

    /** Report whether the caller meets a permission's elevation. */
    elevates(permission: PermissionReference): boolean {
        const elevation = this.#authorizer.elevated.get(PermissionReference.key(permission));

        return elevation === undefined || Elevation.admits(elevation, this.context);
    }

    /** Read the scope's own object from its containing scope when a mapping has its type. */
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

/** Refuse copies of a chain's access with a home silent for longer than the lag. */
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
    const ids = rows.map((row) => [row.id]);
    const objects = rows.map((row) => principal.role.reference(row.scope, row.id));
    const [permissions, relationships] = await Promise.all([
        snapshot.select(accessRolePermission, ["roleId"], ids),
        Relationship.readByObject(snapshot, objects),
    ]);

    // keep the current inclusions among those roles
    const byRole = Map.groupBy(permissions, (row) => row.roleId);
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
        id: row.id,
        isUniversal: row.isUniversal,
        permissions: (byRole.get(row.id) ?? []).map((granted) => ({
            packageId: granted.packageId,
            type: granted.type,
            name: granted.name,
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

/** Read the sets a field makes some subjects members of. */
async function fieldSets(
    snapshot: Snapshot,
    entry: FieldRelation,
    subjects: readonly Subject[],
): Promise<Subject[]> {
    // read the rows with the subjects the field may have
    const holders = subjects.filter((subject) => isAcceptedBy(entry, subject));
    if (holders.length === 0) {
        return [];
    }
    const { mapping, field } = entry;
    const rows = await snapshot.select(
        mapping.table,
        [field.column],
        [...new Set(holders.map((subject) => subject.id))].map((id) => [id]),
    );

    // keep the rows with a subject of their scope, or of any scope the field names
    const definition = mapping.policy.definition;
    const keys = new Set(holders.map((holder) => Subject.key(holder)));

    return rows.flatMap((row) => {
        const scope = TableMapping.scope(mapping, row);
        const subject = {
            ...entry.subject,
            scope: field.scope ?? scope,
            id: TableMapping.text(row, field.column),
        };

        return keys.has(Subject.key(subject))
            ? [
                  {
                      packageId: definition.packageId,
                      type: definition.name,
                      scope,
                      id: TableMapping.text(row, mapping.id),
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

/** Decide whether a field accepts a subject: one of the subject type it accepts, in its scope. */
function isAcceptedBy(entry: FieldRelation, subject: Subject): boolean {
    return (
        subject.packageId === entry.subject.packageId &&
        subject.type === entry.subject.type &&
        subject.relation === entry.subject.relation &&
        (entry.field.scope === undefined || entry.field.scope === subject.scope)
    );
}
