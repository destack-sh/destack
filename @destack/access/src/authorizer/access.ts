import type { DatabaseConnection, Row, Select, Snapshot } from "@destack/db";
import { Replica, type ScopeLink, Scope, type ObjectReference, Subject } from "@destack/sync";
import { found } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../declare/policy.ts";
import { type SubjectType } from "../declare/subject.ts";
import { CONTAINED } from "../declare/expression.ts";
import { ACCESS_PACKAGE_ID } from "../declare/principal.ts";
import * as principal from "../declare/principal.ts";
import { AccessContext, Caller, Contact } from "../context/context.ts";
import { Elevation } from "../context/elevation.ts";
import { GrantPermission } from "../grant/permission.ts";
import { Grant } from "../grant/grant.ts";
import { Relationship } from "../relationship/relationship.ts";
import type { RelationshipRow, RoleBinding } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { Role } from "../role/role.ts";
import type { RoleGrant } from "../role/closure.ts";
import { CHAIN_SHAPE } from "../replica/replica.ts";
import { Authority } from "./authority.ts";
import { TupleCondition, type ConditionContext } from "./condition.ts";
import type { Gate } from "./decision.ts";
import { Authorizer } from "./authorizer.ts";
import { type FieldRelation, TableMapping } from "./mapping.ts";

/** The permission key the roles granting every permission but reserved ones are listed under. */
export const UNIVERSAL = "*";

/** A caller's access in one scope: authority, scope chain, roles along it and validity time. */
export class Access {
    /** The scope every protected row of a query belongs to. */
    readonly scope: string;
    /** The caller. */
    readonly context: AccessContext;
    /** The represented subject's identities and subject sets. */
    readonly authority: Authority;
    /** The objects of the scope and every scope enclosing it, nearest first. */
    readonly scopes: readonly ObjectReference[];
    /** What each role along the scope chain grants, directly or through included roles. */
    readonly grants: ReadonlyMap<string, RoleGrant>;
    /** Whether the scope or an enclosing one is suspended, withholding all but administration. */
    readonly isSuspended: boolean;
    /** Whether the scope's or an enclosing scope's storage is capped, refusing all but deletes. */
    readonly isCapped: boolean;
    /** The fenced scope nearest in the chain and the machine it moves to. */
    readonly moved: { readonly scope: string; readonly machine: string } | undefined;
    /** The partition of the scope's objects, absent for a scope this database lacks. */
    readonly partition: string | undefined;
    /** The next moment time changes the caller's relationships, sets, roles or elevation. */
    readonly until: number | undefined;
    /** The chain's and the caller's roles by permission, as one JSON list of `[permission. */
    readonly grantedRoles: string;
    /** The role bindings of the caller's authority, wherever the objects they bind on live. */
    readonly bindings: readonly RoleBinding[];
    /** The chain's and the caller's roles granting each permission, by permission key. */
    readonly #roles: ReadonlyMap<string, readonly string[]>;
    /** The chain's and the caller's roles granting all but reserved permissions, as owners' do. */
    readonly #universal: readonly string[];
    /** The roles held outside the scope chain, each with the bindings it grants through alone. */
    readonly #confined: ReadonlyMap<string, ReadonlySet<string>>;
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
            readonly authority: Authority;
            readonly links: readonly ScopeLink[];
            readonly grants: ReadonlyMap<string, RoleGrant>;
            readonly confined: ReadonlyMap<string, ReadonlySet<string>>;
            readonly bindings: readonly RoleBinding[];
            readonly until: number | undefined;
        },
    ) {
        // retain what the resolution read
        this.#authorizer = authorizer;
        this.scope = resolved.scope;
        this.context = resolved.context;
        this.authority = resolved.authority;
        this.#links = resolved.links;
        this.scopes = resolved.links.map((link) => link.object);
        this.grants = resolved.grants;
        this.bindings = resolved.bindings;
        this.isSuspended = resolved.links.some((link) => link.isSuspended);
        this.isCapped = resolved.links.some((link) => link.isCapped);
        const fenced = Scope.fenced(resolved.links);
        this.moved =
            fenced?.movedTo === undefined
                ? undefined
                : { scope: fenced.object.id, machine: fenced.movedTo };
        this.partition = resolved.links[0]?.partition;
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
        this.#confined = resolved.confined;

        // list each role under its permission keys, a confined role once per binding it grants through
        const entries = (key: string, role: string) => {
            const confined = resolved.confined.get(role);

            return confined === undefined
                ? [[key, role]]
                : [...confined].map((binding) => [key, role, binding]);
        };
        this.grantedRoles = JSON.stringify([
            ...[...roles].flatMap(([key, granted]) =>
                granted.flatMap((role) => entries(key, role)),
            ),
            ...universal.flatMap((role) => entries(UNIVERSAL, role)),
        ]);
    }

    /** Resolve a caller's access in a scope as a snapshot shows it. */
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

        // refuse a credential bound to a scope the chain leaves out
        const confined = context.credential?.scope;
        if (confined !== undefined && !chain.includes(confined)) {
            throw new AccessError(
                "UNAUTHORIZED",
                `the caller's credential acts within ${confined} alone`,
            );
        }

        // refuse delegates acting under a grant this database keeps and no longer has live
        Caller.requireChain(context);
        const { subject, grant } = context;
        const sender = Caller.sender(context);
        if (
            Caller.isDelegated(context) &&
            subject !== undefined &&
            sender !== undefined &&
            grant !== undefined &&
            (await authorizer.keeps(snapshot, grant.scope))
        ) {
            const kept = await Grant.read(snapshot, grant.id);
            Grant.require(kept, grant.id, { delegator: subject, delegate: sender }, context.now);
        }

        // read the subject sets of the caller with their containing scopes
        const [, represented] = await Promise.all([
            snapshot.position === undefined
                ? requireConfirmed(snapshot.database, chain, authorizer.lag, context.now)
                : undefined,
            Access.expand(
                snapshot,
                await Access.#represented(snapshot, context, scopes),
                context,
                authorizer,
                scopes,
            ),
        ]);

        // read the chain's roles and the roles bound to the caller
        const bindings = represented.bindings;
        const roles = await readRoles(snapshot, chain, bindings, context);

        // take the earliest moment a subject set, an inclusion or an elevation the caller has changes by time
        const until = earliest([
            roles.until,
            represented.until,
            ...[...authorizer.elevated.values()].map((elevation) =>
                Elevation.until(elevation, context),
            ),
        ]);

        return new Access(authorizer, {
            scope,
            context,
            authority: new Authority(represented.subjects),
            links,
            grants: roles.grants,
            confined: roles.confined,
            bindings,
            until,
        });
    }

    /** List the caller's represented subjects: its own, its contacts', and their scopes' sets. */
    static async #represented(
        snapshot: Snapshot,
        context: AccessContext,
        scopes: readonly ObjectReference[],
    ): Promise<Subject[]> {
        return [
            ...context.subjects,
            ...Caller.contacts(context).map((contact) => Contact.subject(contact)),
            ...(await Access.#contained(snapshot, context.subjects, scopes)),
        ];
    }

    /** Resolve the caller in scopes this one encloses, in reads shared by all of them. */
    async descend(snapshot: Snapshot, scopes: readonly string[]): Promise<Map<string, Access>> {
        // link each scope up to this one, leaving out the scopes it does not enclose
        const rows = await this.#readBetween(snapshot, scopes);
        const below = new Map<string, ScopeLink[]>();
        for (const scope of new Set(scopes)) {
            const links = this.#linksUp(rows, scope);
            if (links !== undefined) {
                below.set(scope, links);
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
                ? requireConfirmed(snapshot.database, ids, this.#authorizer.lag, this.context.now)
                : undefined,
        ]);
        const defined = new Set(defining.map((row) => row.scope));

        // resolve each scope below this one
        const resolved = new Map<string, Access>();
        for (const [scope, links] of below) {
            resolved.set(scope, await this.#below(snapshot, scope, links, defined));
        }

        return resolved;
    }

    /** Read the rows of some scopes and of the scopes between them and this scope's chain. */
    async #readBetween(
        snapshot: Snapshot,
        scopes: readonly string[],
    ): Promise<Map<string, Select<typeof Scope.table>>> {
        // read up from the scopes until this scope's chain
        const chain = new Set(this.#links.map((link) => link.object.id));
        const rows = new Map<string, Select<typeof Scope.table>>();
        for (let wanted = scopes.filter((id) => !chain.has(id)); wanted.length > 0;) {
            // read the wanted rows and their parents
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

        return rows;
    }

    /** Link a scope up to this one, absent for a scope this one does not enclose. */
    #linksUp(
        rows: ReadonlyMap<string, Select<typeof Scope.table>>,
        scope: string,
    ): ScopeLink[] | undefined {
        // walk the parents once each
        const links: ScopeLink[] = [];
        const visited = new Set<string>();
        for (
            let current = rows.get(scope);
            current !== undefined && !visited.has(current.scope);
            current = rows.get(current.parent)
        ) {
            // link the row, ending at this scope
            visited.add(current.scope);
            links.push(Scope.link(current));
            if (current.parent === this.scope) {
                return links;
            }
        }

        return undefined;
    }

    /** Resolve the caller in one enclosed scope, reusing these roles unless the chain adds more. */
    async #below(
        snapshot: Snapshot,
        scope: string,
        links: readonly ScopeLink[],
        defined: ReadonlySet<string>,
    ): Promise<Access> {
        // expand the authority again for a chain whose scopes take sets from the scopes enclosing them
        const chain = [...links, ...this.#links];
        const isEnclosed = links.some((link) => this.#authorizer.isEnclosed(link.object));
        const expanded = isEnclosed
            ? await Access.expand(
                  snapshot,
                  this.authority.subjects,
                  this.context,
                  this.#authorizer,
                  chain.map((link) => link.object),
              )
            : undefined;
        const authority =
            expanded === undefined ? this.authority : new Authority(expanded.subjects);
        const bindings = expanded === undefined ? this.bindings : expanded.bindings;

        // read the roles of a chain that defines more, or of new bindings
        const isDefining =
            links.some((link) => defined.has(link.object.id)) ||
            bindings.some((binding) => !this.bindings.some((known) => known.id === binding.id));
        const roles = isDefining
            ? await readRoles(
                  snapshot,
                  chain.map((link) => link.object.id),
                  bindings,
                  this.context,
              )
            : { grants: this.grants, confined: this.#confined, until: undefined };

        return new Access(this.#authorizer, {
            scope,
            context: this.context,
            authority,
            links: chain,
            grants: roles.grants,
            confined: roles.confined,
            bindings,
            until: earliest([this.until, roles.until, expanded?.until]),
        });
    }

    /** Add the subject sets of some subjects breadth first. */
    static async expand(
        snapshot: Snapshot,
        subjects: readonly Subject[],
        context: ConditionContext,
        authorizer: Pick<Authorizer, "fields" | "memberships" | "implied" | "enclosed">,
        chain: readonly ObjectReference[],
    ): Promise<{
        readonly subjects: Subject[];
        readonly bindings: RoleBinding[];
        readonly until: number | undefined;
    }> {
        // admit the identities and verified subject sets, with the sets they reach on the chain
        const expanded = new Map<string, Subject>();
        const bindings = new Map<string, RoleBinding>();
        const moments: (number | undefined)[] = [];
        let frontier = Access.#admit(subjects, expanded, authorizer, chain);
        while (frontier.length > 0) {
            // read the frontier subjects' relationships and the sets their fields reach, together
            const level = frontier;
            const [held, ...fielded] = await Promise.all([
                Relationship.readBySubject(snapshot, level),
                ...authorizer.fields.map((entry) => fieldSets(snapshot, entry, level)),
            ]);

            // split the relationships the request's conditions admit into memberships and bindings
            const current = held.filter(
                (row) => new TupleCondition(row).failure(context) === undefined,
            );
            const related = current.filter((row) => isMembership(row, authorizer.memberships));
            const bound = current.filter(Relationship.isBinding);

            // keep the role bindings of the frontier subjects
            for (const binding of bound) {
                bindings.set(binding.id, binding);
            }

            // note when each relationship of the frontier subjects next starts or stops applying
            moments.push(...held.map((row) => TupleCondition.until(row, context)));

            // continue from each relationship's set and the permissions accepted as subject sets it decides
            const reached = [
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
            frontier = Access.#admit(reached, expanded, authorizer, chain);
        }

        return {
            subjects: [...expanded.values()],
            bindings: [...bindings.values()],
            until: earliest(moments),
        };
    }

    /** List the containment sets of each principal's scopes and the scopes enclosing them. */
    static async #contained(
        snapshot: Snapshot,
        subjects: readonly Subject[],
        chain: readonly ObjectReference[],
    ): Promise<Subject[]> {
        const sets: Subject[] = [];
        for (const subject of subjects) {
            // skip subject sets and wildcards
            if (subject.relation !== undefined || subject.id === "*") {
                continue;
            }

            // find the scopes below the universe the principal lives in or is
            const place = chain.findIndex(
                (scope) => scope.id === subject.scope || scope.id === subject.id,
            );
            const enclosing =
                place !== -1
                    ? chain.slice(place)
                    : subject.scope === Scope.universe.id
                      ? []
                      : (await Scope.chain(snapshot, subject.scope)).map((link) => link.object);
            sets.push(
                ...enclosing
                    .filter((scope) => scope.id !== Scope.universe.id)
                    .map((scope) => ({ ...scope, relation: CONTAINED })),
            );
        }

        return sets;
    }

    /** Add unseen subjects to an expansion with their sets on enclosed scopes, returning them. */
    static #admit(
        reached: readonly Subject[],
        expanded: Map<string, Subject>,
        authorizer: Pick<Authorizer, "enclosed">,
        chain: readonly ObjectReference[],
    ): Subject[] {
        // walk the reached subjects and the sets they reach, in order
        const added: Subject[] = [];
        const pending = [...reached];
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

    /** Report whether a permission is reserved, which owners' roles leave out. */
    isReserved(permission: PermissionReference): boolean {
        return this.#authorizer.reserved.has(PermissionReference.key(permission));
    }

    /** Decide whether a role binding grants its permission. */
    binds(role: {
        readonly id: string;
        readonly binding: string;
        readonly permission: PermissionReference | null;
    }): boolean {
        // find the role among the universal roles for ownership, or among those granting the permission
        const confined = this.#confined.get(role.id);
        const isGranting =
            role.permission === null
                ? this.#universal.includes(role.id)
                : this.#grants(role.permission, role.id);

        return isGranting && (confined === undefined || confined.has(role.binding));
    }

    /** Report whether a role grants a permission, universally unless the permission is reserved. */
    #grants(permission: PermissionReference, roleId: string): boolean {
        const listed = this.#roles.get(PermissionReference.key(permission)) ?? [];
        const isUniversal = !this.isReserved(permission) && this.#universal.includes(roleId);

        return listed.includes(roleId) || isUniversal;
    }

    /** Read the gate a request fails on a row before grants: credential, elevation, suspension. */
    gate(permission: PermissionReference, mapping: TableMapping, row: Row): Gate | undefined {
        return this.admits(permission, TableMapping.text(row, mapping.id), { mapping, row })
            ? this.blocked(permission)
            : "grant";
    }

    /** Decide whether the credential allows a permission on an object, directly or by source. */
    admits(
        permission: PermissionReference,
        id: string,
        stored?: {
            readonly mapping: TableMapping;
            readonly row: Row;
        },
    ): boolean {
        // allow a permission the credential names
        if (
            GrantPermission.allows(
                permission,
                { scope: this.scope, id },
                this.context.grant?.permissions,
            )
        ) {
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

    /** The state of the scope chain: its scopes, suspension, cap, fence and partition, as a key. */
    get chainKey(): string {
        return JSON.stringify([
            this.scopes.map((scope) => scope.id),
            this.isSuspended,
            this.isCapped,
            this.moved ?? null,
            this.partition ?? null,
        ]);
    }

    /** Read the scope's own object from its containing scope, when a mapping has its type. */
    scopeObject(mapping: TableMapping): ObjectReference | undefined {
        // match the containing scope's object by the mapped type
        const object = this.scopes[0];
        const definition = mapping.policy.definition;
        const isMapped =
            object !== undefined &&
            object.packageId === definition.packageId &&
            object.type === definition.name;

        return isMapped ? object : undefined;
    }
}

/** Refuse copies of a chain's access with a home silent for longer than the lag. */
async function requireConfirmed(
    database: DatabaseConnection,
    chain: readonly string[],
    lag: number,
    now: number,
): Promise<void> {
    for (const [scope, origin] of await Replica.origins(database, CHAIN_SHAPE, chain)) {
        if (now - origin.confirmedAt > lag) {
            throw new AccessError(
                "STALE",
                `access of ${scope} was last confirmed by its home ${now - origin.confirmedAt} ms ago`,
            );
        }
    }
}

/** Read a scope chain's roles and the caller's bound roles, closed over their inclusions. */
async function readRoles(
    snapshot: Snapshot,
    chain: readonly string[],
    bindings: readonly RoleBinding[],
    context: ConditionContext,
): Promise<{
    readonly grants: Map<string, RoleGrant>;
    readonly confined: ReadonlyMap<string, ReadonlySet<string>>;
    readonly until: number | undefined;
}> {
    // read the chain's roles, the caller's bound roles, and the chains of those bindings' scopes
    const [chained, held, bound] = await Promise.all([
        snapshot.select(
            accessRole,
            ["scope"],
            chain.map((id) => [id]),
        ),
        snapshot.select(
            accessRole,
            ["id"],
            [...new Set(bindings.map((binding) => binding.roleId))].map((id) => [id]),
        ),
        Scope.chains(snapshot, [...new Set(bindings.map((binding) => binding.objectScope))]),
    ]);

    // confine a bound role outside the chain to bindings at or below its scope
    const chainedIds = new Set(chained.map((row) => row.id));
    const confined = new Map<string, ReadonlySet<string>>();
    for (const role of held.filter((each) => !chainedIds.has(each.id))) {
        const admitted = bindings.filter(
            (binding) =>
                binding.roleId === role.id &&
                (role.scope === binding.objectId ||
                    role.scope === binding.objectScope ||
                    found(bound, binding.objectScope).some(
                        (link) => link.object.id === role.scope,
                    )),
        );
        if (admitted.length > 0) {
            confined.set(role.id, new Set(admitted.map((binding) => binding.id)));
        }
    }
    const rows = [...chained, ...held.filter((role) => confined.has(role.id))];

    // read their permissions, and the relationships on them
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
            new TupleCondition(row).failure(context) === undefined,
    );

    // close the roles over their inclusions
    const roles = rows.map((row): Pick<Role, "id" | "isUniversal" | "permissions"> => ({
        id: row.id,
        isUniversal: row.isUniversal,
        permissions: (byRole.get(row.id) ?? []).map((granted) => ({
            packageId: granted.packageId,
            type: granted.type,
            name: granted.name,
        })),
    }));

    return {
        confined,
        grants: Role.close(
            roles,
            includes.map((include) => ({
                role: include.objectId,
                included: include.subjectId,
            })),
        ),
        until: earliest(includes.map((include) => TupleCondition.until(include, context))),
    };
}

/** Decide whether a relationship makes its subject a member of a set of some or all types. */
function isMembership(row: RelationshipRow, sets: readonly SubjectType[] | undefined): boolean {
    return (
        row.relation !== null &&
        (sets === undefined ||
            sets.some(
                (set) =>
                    set.packageId === row.packageId &&
                    set.type === row.type &&
                    set.relation === row.relation,
            ))
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
