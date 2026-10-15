import * as access from "@destack/access";
import { Replica, Scope, type ObjectReference } from "@destack/sync";
import {
    AccessError,
    earliest,
    AccessContext,
    Caller,
    type Authorizer,
    type TupleReader,
    type Access,
    type Permission,
} from "@destack/access";
import {
    and,
    Key,
    or,
    sql,
    TABLE,
    type SQL,
    Snapshot,
    type Row,
    type Transaction,
} from "@destack/db";
import { aligned, schema } from "@destack/schema";
import { conceal, CallError, isCallError } from "@destack/runtime/error";
import type { MethodContext } from "../method/context.ts";
import { EventLog } from "../query/event.ts";
import type { StoreExecution } from "../method/execution.ts";
import type { MethodKind } from "../method/kind.ts";
import { SCOPE_LIST, SCOPE_READ, type ObjectType } from "../object/object.ts";

/** The kinds of method a scope with capped storage still runs, since they free storage. */
const FREEING_KINDS: ReadonlySet<MethodKind> = new Set(["delete", "deleteMany", "purge"]);

/** A caller's authorization in one call's transaction. */
export class Authorization extends access.Authorization {
    /** The caller's resolved access in the call's scope. */
    readonly access: Access;

    /** Authorize a caller in one transaction. */
    constructor(
        authorizer: Authorizer,
        transaction: Transaction,
        context: () => AccessContext,
        resolved: Access,
    ) {
        super(authorizer, transaction, context, resolved);
        this.access = resolved;
    }

    /** Match the rows the caller may list. */
    listable(object: ObjectType, permission: Permission): SQL | "memory" {
        // decide ephemeral and external rows in memory
        if (object.storage !== "durable") {
            return "memory";
        }

        // match permitted rows and the replicas handed down or decided for the caller below the scope
        const table = object.table;
        const permitted = this.authorizer.where(permission, this.access, table);
        const isReplica = sql<boolean>`${table[TABLE].column("scope")} <> ${this.access.scope}`;
        const caller = Caller.principal(this.context());
        const included = Replica.includes(table, this.access.scope, caller);

        return object.inherited === undefined
            ? or(and(isReplica, included), permitted)
            : or(isReplica, permitted);
    }

    /** Decide which rows the caller may list. */
    async admitRows(
        object: ObjectType,
        permission: Permission,
        scope: string,
        rows: readonly Row[],
        reader?: TupleReader,
    ): Promise<access.Admission & { readonly below: readonly string[] }> {
        // admit the inherited replicas of enclosing scopes wholesale
        const table = object.table;
        const scoped = rows.map((row, position) => ({
            position,
            row,
            scope: schema.string().parse(row["scope"]),
        }));
        const others = scoped.filter((entry) => entry.scope !== scope);
        let replicas: ReadonlySet<number>;
        let below: ReadonlyMap<string, Access> = new Map();
        if (object.inherited !== undefined) {
            replicas = new Set(others.map((entry) => entry.position));
        }
        // admit the caller's durable replicas below the scope, and resolve the other rows' scopes
        else if (object.storage === "durable" && others.length > 0) {
            const included = await Replica.keysIncluded(
                this.transaction.database,
                table,
                others.map((entry) => entry.row),
                scope,
                Caller.principal(this.context()),
            );
            const kept = others.filter((entry) => included.has(Key.name(table, entry.row)));
            replicas = new Set(kept.map((entry) => entry.position));
            const scopes = others
                .filter((entry) => !replicas.has(entry.position))
                .map((entry) => entry.scope);
            below = await this.descend(scope, [...new Set(scopes)]);
        }
        // admit no other scope's rows
        else {
            replicas = new Set();
        }

        // check the caller's permission on the scope's own rows and on the rows of the scopes it encloses
        const own = scoped.filter(
            (entry) =>
                !replicas.has(entry.position) && (entry.scope === scope || below.has(entry.scope)),
        );
        const admission = await this.checkRows(
            permission,
            scope,
            own.map((entry) => entry.row),
            reader,
            below,
        );

        return {
            ...admission,
            permitted: new Set([
                ...replicas,
                ...[...admission.permitted].map((index) => aligned(own, index).position),
            ]),
            below: [...below.keys()],
        };
    }

    /** The call's scope and the scopes containing it, nearest first. */
    get chain(): string[] {
        return [...new Set([this.access.scope, ...this.access.scopes.map((entry) => entry.id)])];
    }

    /** Read a call's target in a snapshot, permitted for the caller now and at a position. */
    async readIn(
        context: MethodContext,
        id: string,
        snapshot: Snapshot,
        permission: Permission = context.permission(),
    ): Promise<Row> {
        // read the row the view shows, admitted with the caller's grants now
        const { object, scope } = context;
        const row = await snapshot.read(object.table).findFirst({ where: { id } });
        const isAdmitted =
            row !== undefined &&
            (await this.admitRows(object, permission, scope, [row])).permitted.has(0);

        // decide the permission as of a position too
        const reference = object.reference(scope, id);
        const governing = this.authorizer.governingScope(reference);
        const isAllowed =
            isAdmitted &&
            (snapshot.position === undefined ||
                (
                    await this.authorizer.check(
                        snapshot,
                        permission,
                        reference,
                        await this.authorizer.resolve(snapshot, governing, this.context()),
                    )
                ).isAllowed);

        // hide a missing row and a row the caller may not read
        if (row === undefined || !isAllowed) {
            throw new CallError("NOT_FOUND", { message: `no ${object.name} ${id}` });
        }

        return row;
    }

    /** Read a call's target where the caller has the method's permission. */
    async read(context: MethodContext, id: string, execution: StoreExecution): Promise<Row> {
        // read where the caller has the permission
        const { object, scope } = context;
        const permission = context.permission();
        const target = object.reference(scope, id);
        const select = (evaluated: Access) =>
            this.#selectPermitted(context, id, permission, evaluated, execution);
        const governing = await this.in(this.authorizer.governingScope(target));
        const [row] = await select(governing);
        if (row) {
            return row;
        }

        // challenge for step-up authentication first and for a grant giving too little second
        const stepUp = await this.authorizer.challenge(
            this.snapshot,
            permission,
            governing,
            async (stepped) => (await select(stepped)).length > 0,
        );
        if (stepUp !== undefined) {
            throw new AccessError(
                "INSUFFICIENT_AUTHENTICATION",
                "authentication assurance is too low: authenticate again",
                { stepUp },
            );
        }
        await this.#requireGranted(context, id, permission, governing, select);

        // refuse a reader, and hide the object from others
        const isReadable =
            object.reading !== undefined &&
            (await this.#permits(context, id, object.reading, execution));
        const denial = new CallError("FORBIDDEN", {
            message: `permission denied: ${permission.name}`,
        });
        throw isReadable ? denial : conceal(denial, `no ${object.name} ${id}`);
    }

    /** Select a call's target where an access has a permission on it. */
    async #selectPermitted(
        context: MethodContext,
        id: string,
        permission: Permission,
        evaluated: Access,
        execution: StoreExecution,
    ): Promise<readonly Row[]> {
        // read the object through the call's transaction, its own writes included, and decide in memory
        return this.#decide(context, id, permission, evaluated, execution);
    }

    /** Refuse a target the caller's subject may access but its grant does not cover. */
    async #requireGranted(
        context: MethodContext,
        id: string,
        permission: Permission,
        governing: Access,
        select: (access: Access) => Promise<readonly Row[]>,
    ): Promise<void> {
        // pass a caller under no grant
        const { object, scope } = context;
        const { grant, ...subject } = { ...governing.context, actors: [] };
        if (grant === undefined) {
            return;
        }

        // refuse the grant when only the subject can access the target
        const governingScope = this.authorizer.governingScope(object.reference(scope, id));
        const alone = await this.authorizer.resolve(this.snapshot, governingScope, subject);
        if ((await select(alone)).length > 0) {
            throw new CallError("INSUFFICIENT_GRANT", {
                message: `grant ${grant.id} does not give ${permission.name} on ${object.name} ${id}`,
                data: { permission, object: object.reference(scope, id), grant: grant.id },
            });
        }
    }

    /** Hide a refusal in a scope the caller cannot see behind the scope's absence. */
    async concealDenial(error: unknown, objects: readonly ObjectType[]): Promise<unknown> {
        // keep anything but a missing or hidden object and a raw access denial
        const isMissing = isCallError(error) && error.code === "NOT_FOUND";
        const isRefused = error instanceof access.AccessError && error.code === "FORBIDDEN";
        if (!(isMissing || isRefused) || (await this.sees(objects))) {
            return error;
        }

        return new CallError("NOT_FOUND", {
            message: `no scope ${this.access.scope}`,
            cause: error,
        });
    }

    /** Refuse a caller listing in a scope it cannot see, as the scope's absence. */
    async requireSeen(objects: readonly ObjectType[]): Promise<void> {
        if (!(await this.sees(objects))) {
            throw new CallError("NOT_FOUND", { message: `no scope ${this.access.scope}` });
        }
    }

    /** Decide whether the caller sees the admitted scope, as the universe everyone sees. */
    async sees(objects: readonly ObjectType[]): Promise<boolean> {
        // see the universe
        const scope = this.access.scope;
        if (scope === Scope.universe.id) {
            return true;
        }

        // decide for the caller, or only for its subject where its grant gives something in the scope
        const own = this.access.scopes[0];
        const { grant, ...subject } = { ...this.access.context, actors: [] };
        const isScope = own?.id === scope && objects.some((object) => object.livesIn(own));

        return (
            isScope &&
            (grant === undefined
                ? await this.#sees(own, (bound) => this.in(bound))
                : grant.permissions.some((entry) => entry.scope === scope) &&
                  (await this.#sees(own, (bound) =>
                      this.authorizer.resolve(this.snapshot, bound, subject),
                  )))
        );
    }

    /** Decide whether an access sees a scope: it reads or lists it, or has a grant inside it. */
    async #sees(
        own: ObjectReference,
        resolve: (scope: string) => Promise<Access>,
    ): Promise<boolean> {
        // see a scope the access reads, or lists where its type declares listing
        const container = await resolve(this.authorizer.governingScope(own));
        const declared = this.authorizer.policy(own).definition.permissions;
        const names = Object.hasOwn(declared, SCOPE_LIST) ? [SCOPE_READ, SCOPE_LIST] : [SCOPE_READ];
        for (const name of names) {
            const permission = { packageId: own.packageId, type: own.type, name };
            const decided = await this.authorizer.check(this.snapshot, permission, own, container);
            if (decided.isAllowed) {
                return true;
            }
        }

        // see a scope the access has a granted relationship inside, such as a shared object
        return this.authorizer.isRelatedIn(this.snapshot, own.id, await resolve(own.id));
    }

    /** Decide whether an object type's rows live in scopes of the admitted scope's type. */
    isScopeOf(object: ObjectType): boolean {
        const own = this.access.scopes[0];

        return this.access.scope === Scope.universe.id
            ? [object.scope].flat().includes(Scope.universe.id)
            : own?.id === this.access.scope && object.livesIn(own);
    }

    /** Decide whether the admitted scope has every row of a type its replicas read. */
    isHomeOf(object: ObjectType): boolean {
        return this.access.scope === Scope.universe.id
            ? object.scope === Scope.universe.id
            : this.isScopeOf(object);
    }

    /** Refuse a type routed outside the admitted scope. */
    requireScopeOf(object: ObjectType): void {
        if (!this.isScopeOf(object)) {
            throw new CallError("NOT_FOUND", {
                message: `no ${object.plural} in scope ${this.access.scope}`,
            });
        }
    }

    /** Refuse a moved scope with its new holder, unless every permission is a moving one. */
    requireUnmoved(permissions: readonly access.PermissionReference[]): void {
        const moved = this.access.moved;
        const isMoving =
            permissions.length > 0 &&
            permissions.every((permission) =>
                this.authorizer.moving.has(access.PermissionReference.key(permission)),
            );
        if (moved !== undefined && !isMoving) {
            throw new CallError("MISDIRECTED_REQUEST", {
                message: `${moved.scope} moves to ${moved.machine}`,
                data: moved,
            });
        }
    }

    /** Refuse a call in another partition than the store's, unless it serves every partition. */
    requirePartition(served: string | undefined): void {
        const { scope, partition } = this.access;
        if (served !== undefined && partition !== undefined && partition !== served) {
            throw new CallError("MISDIRECTED_REQUEST", {
                message: `${scope} is served by partition ${partition}`,
                data: { scope, partition },
            });
        }
    }

    /** Refuse writes to a scope with capped storage: every mutating call but delete or purge. */
    requireUncapped(
        calls: readonly { readonly object: ObjectType; readonly name: string }[],
    ): void {
        // admit any call in a scope within its storage
        if (!this.access.isCapped) {
            return;
        }

        // refuse the first call that writes without freeing storage
        const writing = calls.find(({ object, name }) => {
            const { kind, mutates } = object.method(name);

            return mutates && !FREEING_KINDS.has(kind);
        });
        if (writing !== undefined) {
            throw new CallError("QUOTA_EXCEEDED", {
                message: `storage of scope ${this.access.scope} is capped: ${writing.object.name}.${writing.name} is refused until data is deleted or the plan changes`,
            });
        }
    }

    /** Require the write permission of every guarded field a call sets. */
    async requireWritable(
        context: MethodContext,
        id: string,
        execution: StoreExecution,
    ): Promise<void> {
        for (const [name, declared] of Object.entries(context.object.fields)) {
            const permission = declared.access?.write;
            if (
                permission !== undefined &&
                Object.hasOwn(context.input, name) &&
                !(await this.#permits(
                    context,
                    id,
                    context.object.permission(permission),
                    execution,
                ))
            ) {
                throw new CallError("FORBIDDEN", { message: `field ${name} is not writable` });
            }
        }
    }

    /** Decide whether the caller has a permission on one object of a call's type. */
    async #permits(
        context: MethodContext,
        id: string,
        permission: access.PermissionReference,
        execution: StoreExecution,
    ): Promise<boolean> {
        const { object, scope } = context;
        if (object.storage !== "durable") {
            const granted = await this.in(
                this.authorizer.governingScope(object.reference(scope, id)),
            );

            return (await this.#decide(context, id, permission, granted, execution)).length > 0;
        }

        return (await this.check(permission, object.reference(scope, id))).isAllowed;
    }

    /** Read an ephemeral or external object the access has a permission on, decided in memory. */
    async #decide(
        context: MethodContext,
        id: string,
        permission: access.PermissionReference,
        evaluated: Access,
        execution: StoreExecution,
    ): Promise<Row[]> {
        // read the row in the call's scope: an event from the event log, any other through the transaction
        const row =
            context.object.storage !== "events"
                ? await execution.transaction
                      .read(context.object.table)
                      .findFirst({ where: { id } })
                : await EventLog.row(
                      context,
                      execution.store.events,
                      execution.transaction.database,
                      id,
                  );
        const rows: readonly Row[] =
            row === undefined || row["scope"] !== context.scope ? [] : [row];

        // decide the row read by its key in the call's scope
        const reader = this.authorizer.reader(this.snapshot, evaluated.scopes);
        const { permitted } = await this.authorizer.checkRows(
            this.snapshot,
            permission,
            evaluated,
            rows,
            reader,
            new Map(),
            "object",
        );

        return rows.filter((_, position) => permitted.has(position));
    }

    /** Keep the rows the caller also had a permission on at a snapshot's position. */
    async keepAt<Kept extends Row>(
        rows: readonly Kept[],
        permission: access.PermissionReference,
        snapshot: Snapshot,
        scope: string,
    ): Promise<Kept[]> {
        const then = await this.authorizer.resolve(snapshot, scope, this.context());
        const { permitted } = await this.authorizer.checkRows(snapshot, permission, then, rows);

        return rows.filter((_, position) => permitted.has(position));
    }

    /** Keep the rows an access has a permission on, decided in memory. */
    async keep<Kept extends Row>(
        rows: readonly Kept[],
        permission: access.PermissionReference,
        evaluated: Access = this.access,
    ): Promise<Kept[]> {
        const { permitted } = await this.authorizer.checkRows(
            this.snapshot,
            permission,
            evaluated,
            rows,
        );

        return rows.filter((_, position) => permitted.has(position));
    }

    /** List each row's guarded fields the caller may not read, and until when. */
    async concealed(
        object: ObjectType,
        rows: readonly Row[],
        reader?: TupleReader,
    ): Promise<{ readonly hidden: string[][]; readonly until?: number }> {
        // group the guarded fields by read permission
        const hidden = rows.map((): string[] => []);
        const permissions = new Map<string, string[]>();
        for (const [name, field] of Object.entries(object.fields)) {
            const permission = field.access?.read;
            if (permission !== undefined) {
                permissions.set(permission, [...(permissions.get(permission) ?? []), name]);
            }
        }

        // decide the rows of the scopes below in their own chains
        const scope = this.access.scope;
        const others = rows
            .map((row) => schema.string().parse(row["scope"]))
            .filter((other) => other !== scope);
        const below =
            permissions.size === 0 || others.length === 0
                ? undefined
                : await this.descend(scope, [...new Set(others)]);

        // check each distinct read permission once over every row read by its key
        const moments: (number | undefined)[] = [];
        for (const [permission, names] of permissions) {
            const readable = await this.checkRows(
                object.permission(permission),
                scope,
                rows,
                reader,
                below,
                "object",
            );
            moments.push(readable.until);
            for (const [position, fields] of hidden.entries()) {
                if (!readable.permitted.has(position)) {
                    fields.push(...names);
                }
            }
        }
        const until = earliest(moments);

        // list each row's hidden fields in declaration order
        const order = Object.keys(object.fields);
        const ordered = hidden.map((fields) => order.filter((name) => fields.includes(name)));

        return { hidden: ordered, ...(until === undefined ? {} : { until }) };
    }

    /** Conceal the guarded fields the caller may not read, reporting personal values shown. */
    async redact(
        object: ObjectType,
        rows: readonly Row[],
    ): Promise<{ readonly rows: Row[]; readonly isRevealing: boolean }> {
        // decide the guarded fields the caller may not read on each row
        const table = object.table[TABLE];
        const hidden =
            object.guarded.length === 0
                ? rows.map((): string[] => [])
                : (await this.concealed(object, rows)).hidden;

        return {
            rows: rows.map((row, position) =>
                table.conceal(row, [...object.sensitive, ...aligned(hidden, position)]),
            ),
            isRevealing: object.reveals(rows, hidden),
        };
    }
}
