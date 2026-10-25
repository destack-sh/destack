import {
    type DatabaseConnection,
    Transaction,
    type Row,
    type RowImage,
    type Select,
    type Snapshot,
    Table,
} from "@destack/db";
import type { Environment } from "@destack/environment";
import { ObjectReference, Scope, Subject } from "@destack/sync";

import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../declare/policy.ts";

import { AccessContext } from "../context/context.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";

import { accessInvitation } from "../invitation/table.ts";
import { Manager } from "../manager/manager.ts";
import { Role } from "../role/role.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";

import type { Access } from "./access.ts";
import type { PermittedRows, Authorizer } from "./authorizer.ts";
import type { Decision } from "./decision.ts";
import type { TupleReader, Lookup } from "./tuple.ts";

/** The relationship fields that authorization reads from a request, invitation or stored row. */
type RelationshipFields = {
    /** The related object. */
    readonly object: ObjectReference;
    /** The subject, absent while an offer awaits its recipient. */
    readonly subject?: Subject;
} & ({ readonly relation: string } | { readonly role: string });

/** The first relationships of a new object and the owners of a new scope. */
export interface Creation {
    /** The relationships the object starts with. */
    readonly relationships?: readonly {
        readonly relation: string;
        readonly subject: Subject;
    }[];
    /** The subject of a new scope's owner role, absent if its containing scopes' owners suffice. */
    readonly owner?: Subject;
}

/** The clock, randomness and WebCrypto an authorization creates rows and secrets with. */
export type AuthorizationEnvironment = Pick<Environment, "clock" | "crypto" | "random">;

/** A caller's authorization under a set of policies in one database. */
export class Authorization {
    /** The policies deciding the caller's access. */
    readonly authorizer: Authorizer;
    /** The transaction that reads and writes the access rows. */
    readonly transaction: Transaction;
    /** The database as its decisions read it: the transaction's view, recording each read. */
    readonly snapshot: Snapshot;
    /** The applied declaration writing its rows, null for direct access changes. */
    readonly manager: Manager | null;
    /** The clock, randomness and WebCrypto the call's new rows and secrets come from. */
    readonly environment: AuthorizationEnvironment;
    /** Read the verified caller at the time it acts. */
    readonly #context: () => AccessContext;
    /** The caller's access resolved in each scope during this call, until renewed. */
    readonly #resolved: Map<string, Promise<Access>>;
    /** The caller's access in the scopes below each scope until renewed, absent outside it. */
    readonly #below: Map<string, Map<string, Access | undefined>>;
    /** The transaction's writes to the deciding tables when the resolved access was read. */
    #writes: number;

    /** Authorize one caller in a transaction, with access in some scope. */
    constructor(
        authorizer: Authorizer,
        transaction: Transaction,
        context: () => AccessContext,
        environment: AuthorizationEnvironment,
        options: {
            /** The caller's access already resolved in one scope. */
            readonly resolved?: Access;
            /** The applied declaration writing the rows. */
            readonly manager?: Manager;
        } = {},
    ) {
        // start with empty state
        const { resolved, manager = null } = options;
        this.#resolved = new Map<string, Promise<Access>>();
        this.#below = new Map<string, Map<string, Access | undefined>>();
        this.manager = manager;
        this.environment = environment;
        this.#writes = transaction.changes.writes(authorizer.decidingTables);

        // take the caller, reusing the access resolved in one scope
        this.authorizer = authorizer;
        this.transaction = transaction;
        this.snapshot = transaction.view();
        this.#context = context;
        if (resolved !== undefined) {
            this.#resolved.set(resolved.scope, Promise.resolve(resolved));
        }
    }

    /** Run a call with one caller's authorization in a database transaction until commit. */
    static run<Value>(
        authorizer: Authorizer,
        database: DatabaseConnection,
        context: () => AccessContext,
        environment: AuthorizationEnvironment,
        call: (authorization: Authorization) => Promise<Value>,
    ): Promise<Value> {
        return Transaction.run(database, (transaction) =>
            call(new Authorization(authorizer, transaction, context, environment)),
        );
    }

    /** Authorize the caller to apply a declaration that manages its rows. */
    applying(manager: Manager): Authorization {
        return new Authorization(
            this.authorizer,
            this.transaction,
            this.#context,
            this.environment,
            {
                manager,
            },
        );
    }

    /** Read the verified caller as it acts now. */
    context(): AccessContext {
        return this.#context();
    }

    /** Resolve the caller's access in a scope once per call: subject sets, roles, scope chain. */
    in(scope: string): Promise<Access> {
        // resolve each scope once, until renewed or the deciding tables change
        this.#renewChanged();
        const known = this.#resolved.get(scope);
        if (known !== undefined) {
            return known;
        }
        const resolved = this.authorizer.resolve(this.snapshot, scope, this.context());
        this.#resolved.set(scope, resolved);

        return resolved;
    }

    /** Forget every resolved scope so the next decision reads current access. */
    renew(): void {
        this.#resolved.clear();
        this.#below.clear();
        this.#writes = this.transaction.changes.writes(this.authorizer.decidingTables);
    }

    /** Renew the resolved access once the transaction wrote to the tables deciding it. */
    #renewChanged(): void {
        if (this.transaction.changes.writes(this.authorizer.decidingTables) !== this.#writes) {
            this.renew();
        }
    }

    /** Decide whether the caller has a permission on one object and until when. */
    async check(
        permission: PermissionReference,
        target: ObjectReference,
        reader?: TupleReader,
    ): Promise<Decision> {
        const access = await this.in(this.authorizer.governingScope(target));

        return this.authorizer.check(this.snapshot, permission, target, access, reader);
    }

    /** Start reading tuples for decisions in a scope. */
    async reader(scope: string): Promise<TupleReader> {
        return this.authorizer.reader(this.snapshot, (await this.in(scope)).scopes);
    }

    /** Require the caller to have a permission on one object. */
    async require(permission: PermissionReference, target: ObjectReference): Promise<void> {
        const access = await this.in(this.authorizer.governingScope(target));
        await this.authorizer.require(this.snapshot, [permission], target, access);
    }

    /** Check a permission on a scope's current or past rows. */
    async checkRows(
        permission: PermissionReference,
        scope: string,
        rows: readonly Row[],
        reader?: TupleReader,
        below?: ReadonlyMap<string, Access>,
        lookup?: Lookup,
    ): Promise<PermittedRows> {
        const access = await this.in(scope);

        return this.authorizer.checkRows(
            this.snapshot,
            permission,
            access,
            rows,
            reader,
            below,
            lookup,
        );
    }

    /** Resolve the caller's access in a scope's enclosed scopes, in reads shared by all. */
    async descend(scope: string, below: readonly string[]): Promise<Map<string, Access>> {
        // resolve the scopes not resolved before together
        this.#renewChanged();
        const known = this.#below.get(scope) ?? new Map<string, Access | undefined>();
        this.#below.set(scope, known);
        const missing = [...new Set(below)].filter((enclosed) => !known.has(enclosed));
        if (missing.length > 0) {
            const resolved = await (await this.in(scope)).descend(this.snapshot, missing);
            for (const enclosed of missing) {
                known.set(enclosed, resolved.get(enclosed));
            }
        }

        // collect the scopes the scope encloses
        const enclosing = new Map<string, Access>();
        for (const enclosed of below) {
            const access = known.get(enclosed);
            if (access !== undefined) {
                enclosing.set(enclosed, access);
            }
        }

        return enclosing;
    }

    /** Record a new object's access beside its row: a scope's row, its first subjects and owner. */
    async create(object: ObjectReference, creation: Creation): Promise<void> {
        // record a new scope below the scopes containing it, none below the universe
        const isScope = this.authorizer.policy(object).definition.scope === true;
        if (isScope) {
            const ancestors =
                object.scope === Scope.universe.id
                    ? []
                    : [object.scope, ...(await this.#parent(object.scope)).ancestors];
            await this.#record(object, ancestors);
        }

        // relate its first subjects and owner
        await this.#relate(object, creation);
    }

    /** Record a scope at its home as it is born, below its containing scopes. */
    async receive(object: ObjectReference, ancestors: readonly string[]): Promise<void> {
        await this.#record(object, ancestors);
    }

    /** Forget a removed object's access: its relationships, a scope's row and inside rows. */
    async remove(object: ObjectReference): Promise<void> {
        // require the object's row and access here, in the scope containing it
        const isScope = this.authorizer.policy(object).definition.scope === true;
        const view = this.transaction.view();
        await this.authorizer.requireLocal(this.transaction, object);

        // read the relationships and invitations on the object, and a scope's inside access only at its home
        const on = Relationship.objectColumns(object);
        const related = await this.transaction.read(accessRelationship).findMany({ where: on });
        const invited = await this.transaction.read(accessInvitation).findMany({ where: on });
        const isHome = isScope && (await this.authorizer.isHome(view, object.id));
        const inside = isHome
            ? await Promise.all([
                  this.transaction
                      .read(accessRelationship)
                      .findMany({ where: { scope: object.id } }),
                  this.transaction.read(accessInvitation).findMany({ where: { scope: object.id } }),
                  this.transaction.read(accessRole).findMany({ where: { scope: object.id } }),
              ])
            : undefined;
        const scope = isScope
            ? await this.transaction.read(Scope.table).findFirst({ where: { scope: object.id } })
            : undefined;

        // delete the relationships and invitations once each, then the scope's roles with their permissions
        const relationships = new Map(
            [...related, ...(inside?.[0] ?? [])].map((row) => [row.id, row]),
        );
        const invitations = new Map(
            [...invited, ...(inside?.[1] ?? [])].map((row) => [row.id, row]),
        );
        const roles = inside?.[2] ?? [];
        const permissions = await this.transaction.read(accessRolePermission).findMany({
            where: { roleId: { in: roles.map((row) => row.id) } },
        });
        await this.transaction.write(
            ...[...relationships.values()].map((row) => ({
                table: accessRelationship,
                before: row,
                after: null,
            })),
            ...[...invitations.values()].map((row) => ({
                table: accessInvitation,
                before: row,
                after: null,
            })),
            ...permissions.map((row) => ({
                table: accessRolePermission,
                before: row,
                after: null,
            })),
            ...roles.map((row) => ({ table: accessRole, before: row, after: null })),
        );

        // forget the scope's row last, which the containing scope's home stores
        if (scope !== undefined) {
            await this.transaction.write({ table: Scope.table, before: scope, after: null });
        }
    }

    /** Relate a local object's first subjects, and bind a scope's owner. */
    async #relate(object: ObjectReference, creation: Creation): Promise<void> {
        // write the access of a local object with no relationships yet
        await this.authorizer.requireLocal(this.transaction, object);
        const related = await this.transaction
            .read(accessRelationship)
            .findMany({ where: Relationship.objectColumns(object), limit: 1 });
        if (related.length > 0) {
            throw AccessError.of(
                "INVALID_RELATIONSHIP",
                "a relationship needs a grant after creation",
            );
        }

        // refuse owners of an object that is not a scope
        const definition = this.authorizer.policy(object).definition;
        if (definition.scope !== true && creation.owner !== undefined) {
            throw AccessError.of("SCOPE_REQUIRED", "only a scope can have owners");
        }
        const context = this.context();

        // refuse a first subject set outside the object's scope chain, as a grant refuses it
        const sets = [
            creation.owner,
            ...(creation.relationships ?? []).map((each) => each.subject),
        ].filter((subject) => subject?.relation !== undefined);
        const links = sets.length === 0 ? [] : await Scope.chain(this.snapshot, object.scope);
        for (const subject of sets) {
            await this.#requireInChain(
                object,
                subject,
                links.map((link) => link.object.id),
            );
        }

        // relate the first subjects
        const relationships = this.authorizer.initialRelationships(
            object,
            creation,
            context.now,
            this.environment,
        );
        await this.transaction.write(
            ...relationships.map((relationship) => ({
                table: accessRelationship,
                before: null,
                after: Table.complete(accessRelationship, relationship),
            })),
        );

        // bind the owner role on a scope to its owner
        if (creation.owner !== undefined) {
            await Role.bind(object, Role.OWNER, creation.owner, this);
        }
    }

    /** Record a scope's row below its ancestors, in the scope's partition or its parent's. */
    async #record(object: ObjectReference, ancestors: readonly string[]): Promise<void> {
        // write the row in the scope's partition or its parent's
        const definition = this.authorizer.policy(object).definition;
        const partition =
            definition.partition === true
                ? object.id
                : (await this.#parent(object.scope)).partition;
        const row = Table.complete(Scope.table, {
            scope: object.id,
            parent: object.scope,
            partition,
            packageId: object.packageId,
            type: object.type,
            ancestors: [...ancestors],
        });
        await this.transaction.write({ table: Scope.table, before: null, after: row });
    }

    /** Read the row of a new scope's parent, refusing a scope this database lacks. */
    async #parent(scope: string): Promise<RowImage<typeof Scope.table>> {
        const [row] = await Scope.read(this.snapshot, [scope]);
        if (row === undefined) {
            throw AccessError.of("SCOPE_NOT_FOUND", `unknown scope: ${scope}`);
        }

        return row;
    }

    /** Suspend a scope and withhold every permission in it except administration until resumed. */
    async suspend(object: ObjectReference): Promise<void> {
        await this.#mark(object, { suspendedAt: this.context().now });
    }

    /** Resume a suspended scope, as a caller with that permission. */
    async resume(object: ObjectReference): Promise<void> {
        await this.#mark(object, { suspendedAt: null });
    }

    /** Fence a scope and its subtree for a move to another machine. */
    async fence(object: ObjectReference, machine: string): Promise<void> {
        await this.#mark(object, { fencedAt: this.context().now, movedTo: machine });
    }

    /** Lift a scope's fence on the database storing it. */
    async unfence(object: ObjectReference): Promise<void> {
        await this.#mark(object, { fencedAt: null, movedTo: null });
    }

    /** Require permissions to change roles in a local scope, from fresh caller access. */
    async authorizeRole(
        scope: ObjectReference,
        permissions: readonly PermissionReference[],
        record?: Select<typeof accessRole>,
    ): Promise<void> {
        // write only the roles of a homed scope, and refuse a managed role outside its declaration until detached
        await this.authorizer.requireHome(this.transaction, scope, scope.id);
        if (record !== undefined) {
            Manager.requireChangeable(record, this.manager, "role");
        }

        // require every permission in the scope
        const access = await this.authorizer.resolve(this.snapshot, scope.id, this.context());
        await this.authorizer.require(this.snapshot, permissions, scope, access);
    }

    /** Refuse a subject set outside an object's scope chain, unless its rows live at this home. */
    async #requireInChain(
        object: ObjectReference,
        subject: Subject | undefined,
        chain: readonly string[],
    ): Promise<void> {
        const isOutside =
            subject?.relation !== undefined &&
            subject.scope !== "*" &&
            ![Scope.universe.id, object.scope, ...chain].includes(subject.scope);
        if (isOutside && !(await this.authorizer.isHome(this.snapshot, subject.scope))) {
            throw AccessError.of(
                "SUBJECT_OUTSIDE_CHAIN",
                `${subject.type}#${subject.relation} in ${subject.scope} is outside the scope chain of ${object.scope}`,
            );
        }
    }

    /** Require the grant permission and every permission of a bound role, returning its scope. */
    async authorizeGrant(request: RelationshipFields): Promise<string> {
        // write only a relationship stored here
        const placed = await this.authorizer.requireRelationship(this.transaction, request);

        // require the relation's grant permission to grant it
        const scope = this.authorizer.governingScope(request.object);
        const access = await this.authorizer.resolve(this.snapshot, scope, this.context());
        const role = "role" in request ? request.role : undefined;

        // refuse a subject set outside the object's scope chain, unless its rows live at this home
        const chain = access.scopes.map((link) => link.id);
        await this.#requireInChain(request.object, request.subject, chain);
        const required: PermissionReference[] = [
            this.#grantPermission(request.object, Authorization.#relation(request)),
        ];

        // prevent escalation: bind only scope chain roles with permissions the caller has
        const grants = role === undefined ? undefined : access.grants.get(role);
        if (role !== undefined && grants === undefined) {
            throw AccessError.of("ROLE_NOT_FOUND", "role not found");
        }
        required.push(...(grants?.permissions ?? []));
        await this.authorizer.require(this.snapshot, required, request.object, access);

        // require ownership to bind a role granting everything
        if (
            grants?.isOwner === true &&
            !(await this.authorizer.isOwner(this.snapshot, request.object, access))
        ) {
            throw AccessError.of(
                "OWNER_REQUIRED",
                "only owners may bind a role granting everything",
            );
        }

        return placed;
    }

    /** Require a local object and the grant permission of a relationship. */
    async authorizeRevoke(
        request: RelationshipFields,
        record?: Select<typeof accessRelationship>,
    ): Promise<void> {
        // write only a relationship stored here, and refuse a managed one outside its declaration until detached
        await this.authorizer.requireRelationship(this.transaction, request);
        if (record !== undefined) {
            Manager.requireChangeable(record, this.manager, "relationship");
        }

        // require the relation's grant permission
        await this.requireGrant(request.object, Authorization.#relation(request));
    }

    /** Require the permission granting a relation, or binding roles, on an object. */
    async requireGrant(object: ObjectReference, relation: string | undefined): Promise<void> {
        const scope = this.authorizer.governingScope(object);
        const access = await this.authorizer.resolve(this.snapshot, scope, this.context());
        await this.authorizer.require(
            this.snapshot,
            [this.#grantPermission(object, relation)],
            object,
            access,
        );
    }

    /** Resolve the permission granting a relation, or binding roles, on an object. */
    #grantPermission(object: ObjectReference, relation: string | undefined): PermissionReference {
        // read the relation's grant, or the type's for a role binding
        const policy = this.authorizer.policy(object);
        const grant =
            relation === undefined
                ? policy.definition.grantedBy
                : this.authorizer.relation(policy, relation).grantedBy;
        if (grant === undefined) {
            throw AccessError.of("UNGRANTABLE", "relationship cannot be granted");
        }

        return policy.permission(grant);
    }

    /** Write some columns of a homed scope's record. */
    async #mark(
        object: ObjectReference,
        changes: Partial<Select<typeof Scope.table>>,
    ): Promise<void> {
        // require a scope homed here
        if (this.authorizer.policy(object).definition.scope !== true) {
            throw AccessError.of("SCOPE_REQUIRED", `${object.type} is not a scope`);
        }
        await this.authorizer.requireHome(this.transaction, object, object.id);

        // mark the scope's record
        const row = await this.transaction
            .read(Scope.table)
            .findFirst({ where: { scope: object.id } });
        if (row === undefined) {
            throw AccessError.of("SCOPE_NOT_FOUND", `unknown scope: ${object.id}`);
        }
        await this.transaction.write({
            table: Scope.table,
            before: row,
            after: { ...row, ...changes },
        });
    }

    /** Read the relation a relationship relates through, absent for a role binding. */
    static #relation(request: RelationshipFields): string | undefined {
        return "relation" in request ? request.relation : undefined;
    }
}
