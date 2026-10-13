import * as access from "@destack/access";
import { Replica, Scope, type ObjectReference } from "@destack/sync";
import {
    AccessError,
    earliest,
    AccessContext,
    type Authorizer,
    type TupleReader,
    type Access,
    type Permission,
} from "@destack/access";
import {
    and,
    Key,
    ne,
    or,
    TABLE,
    type SQL,
    Snapshot,
    type Row,
    type Transaction,
} from "@destack/db";
import { aligned, schema } from "@destack/schema";
import { conceal, CallError } from "@destack/runtime/error";
import type { Call } from "../method/call.ts";
import { EventListing } from "../query/event.ts";
import type { MethodKind } from "../method/kind.ts";
import { SCOPE_READ, type ObjectType } from "../object/object.ts";

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

        // match permitted rows and the copies handed down or kept for the scope
        const table = object.table;
        const permitted = this.authorizer.where(permission, this.access, table);
        const isCopy = ne(table[TABLE].column("scope"), this.access.scope);
        const included = Replica.includes(this.access.scope, table);

        return object.inherited === undefined
            ? or(and(isCopy, included), permitted)
            : or(isCopy, permitted);
    }

    /** Decide which rows the caller may list. */
    async admitRows(
        object: ObjectType,
        permission: Permission,
        scope: string,
        rows: readonly Row[],
        reader?: TupleReader,
    ): Promise<access.Admission & { readonly below: readonly string[] }> {
        // admit the inherited copies of enclosing scopes wholesale
        const table = object.table;
        const scoped = rows.map((row, position) => ({
            position,
            row,
            scope: schema.string().parse(row["scope"]),
        }));
        const others = scoped.filter((entry) => entry.scope !== scope);
        let copies: ReadonlySet<number>;
        let below: ReadonlyMap<string, Access> = new Map();
        if (object.inherited !== undefined) {
            copies = new Set(others.map((entry) => entry.position));
        }
        // admit the durable copies of universe rows kept for the scope, and resolve the scopes of the other rows
        else if (object.storage === "durable" && others.length > 0) {
            const included = await Replica.keysIncluded(
                this.transaction.database,
                scope,
                table,
                others.map((entry) => entry.row),
            );
            const kept = others.filter((entry) => included.has(Key.name(table, entry.row)));
            copies = new Set(kept.map((entry) => entry.position));
            const scopes = others
                .filter((entry) => !copies.has(entry.position))
                .map((entry) => entry.scope);
            below = await this.descend(scope, [...new Set(scopes)]);
        }
        // admit no other scope's rows
        else {
            copies = new Set();
        }

        // check the caller's permission on the scope's own rows and on the rows of the scopes it encloses
        const own = scoped.filter(
            (entry) =>
                !copies.has(entry.position) && (entry.scope === scope || below.has(entry.scope)),
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
                ...copies,
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
        call: Call,
        id: string,
        snapshot: Snapshot,
        permission: Permission = call.permission(),
    ): Promise<Row> {
        // read the row the view shows, admitted with the caller's grants now
        const { object, scope } = call;
        const row = await snapshot.row(object.table, { id });
        const isAdmitted =
            row !== null &&
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
    async read(call: Call, id: string): Promise<Row> {
        // read where the caller has the permission
        const { object, scope } = call;
        const permission = call.permission();
        const target = object.reference(scope, id);
        const select = (evaluated: Access) =>
            this.#selectPermitted(call, id, permission, evaluated);
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
                "authenticate again at the required assurance",
                { stepUp },
            );
        }
        await this.#requireGranted(call, id, permission, governing, select);

        // refuse a reader, and hide the object from others
        const isReadable =
            object.reading !== undefined && (await this.#permits(call, id, object.reading));
        const denial = new CallError("FORBIDDEN", {
            message: `permission denied: ${permission.name}`,
        });
        throw isReadable ? denial : conceal(denial, `no ${object.name} ${id}`);
    }

    /** Select a call's target where an access has a permission on it. */
    async #selectPermitted(
        call: Call,
        id: string,
        permission: Permission,
        evaluated: Access,
    ): Promise<readonly Row[]> {
        // read the object through the call's transaction, its own writes included, and decide in memory
        return this.#decide(call, id, permission, evaluated);
    }

    /** Refuse a target the caller's subject may reach and its grant gives too little for. */
    async #requireGranted(
        call: Call,
        id: string,
        permission: Permission,
        governing: Access,
        select: (access: Access) => Promise<readonly Row[]>,
    ): Promise<void> {
        // pass a caller under no grant
        const { object, scope } = call;
        const { grant, delegates: _delegates, ...subject } = governing.context;
        if (grant === undefined) {
            return;
        }

        // refuse the grant when the subject alone reaches the target
        const governingScope = this.authorizer.governingScope(object.reference(scope, id));
        const alone = await this.authorizer.resolve(this.snapshot, governingScope, subject);
        if ((await select(alone)).length > 0) {
            throw new CallError("INSUFFICIENT_GRANT", {
                message: `grant ${grant.id} gives no ${permission.name} on ${object.name} ${id}`,
                data: { permission, object: object.reference(scope, id), grant: grant.id },
            });
        }
    }

    /** Require the call's scope to be visible to the caller. */
    async requireVisible(objects: readonly ObjectType[]): Promise<void> {
        // see the universe
        const scope = this.access.scope;
        if (scope === Scope.universe.id) {
            return;
        }

        // require a scope some of the objects live in
        const own = this.access.scopes[0];
        if (own?.id !== scope || !objects.some((object) => object.livesIn(own))) {
            throw new CallError("NOT_FOUND", { message: `no scope ${scope}` });
        }

        // decide for the caller, or for its subject alone where its grant gives something in the scope
        const { grant, delegates: _delegates, ...subject } = this.access.context;
        const isVisible =
            grant === undefined
                ? await this.#sees(own, objects, (bound) => this.in(bound))
                : grant.permissions.some((entry) => entry.scope === scope) &&
                  (await this.#sees(own, objects, (bound) =>
                      this.authorizer.resolve(this.snapshot, bound, subject),
                  ));
        if (!isVisible) {
            const denial = new CallError("FORBIDDEN", {
                message: `scope ${scope} is not visible`,
            });
            throw conceal(denial, `no scope ${scope}`);
        }
    }

    /** Decide whether an access sees a scope. */
    async #sees(
        own: ObjectReference,
        objects: readonly ObjectType[],
        resolve: (scope: string) => Promise<Access>,
    ): Promise<boolean> {
        // see a readable scope
        const container = await resolve(this.authorizer.governingScope(own));
        const permission = { packageId: own.packageId, type: own.type, name: SCOPE_READ };
        if ((await this.authorizer.check(this.snapshot, permission, own, container)).isAllowed) {
            return true;
        }

        // see a scope with a permitted durable object
        const scope = own.id;
        const evaluated = await resolve(scope);
        const inside = objects.filter(
            (object) => object.storage === "durable" && object.livesIn(own),
        );
        for (const object of inside) {
            const table = object.table;
            const permitted = object.permissions.map((name) =>
                this.authorizer.where(object.permission(name), evaluated, table),
            );
            const [row] = await this.transaction.database
                .select({ id: table[TABLE].column("id") })
                .from(table)
                .where(and(object.inScope(scope), or(...permitted)))
                .limit(1);
            if (row !== undefined) {
                return true;
            }
        }

        return false;
    }

    /** Decide whether an object type's rows live in scopes of the admitted scope's type. */
    isScopeOf(object: ObjectType): boolean {
        const own = this.access.scopes[0];

        return this.access.scope === Scope.universe.id
            ? [object.scope].flat().includes(Scope.universe.id)
            : own?.id === this.access.scope && object.livesIn(own);
    }

    /** Decide whether the admitted scope has every row of a type its copies read. */
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

    /** Refuse a moved scope with its new holder. */
    requireUnmoved(): void {
        const moved = this.access.moved;
        if (moved !== undefined) {
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
    async requireWritable(call: Call, id: string): Promise<void> {
        for (const [name, declared] of Object.entries(call.object.fields)) {
            const permission = declared.access?.write;
            if (
                permission !== undefined &&
                Object.hasOwn(call.input, name) &&
                !(await this.#permits(call, id, call.object.permission(permission)))
            ) {
                throw new CallError("FORBIDDEN", { message: `field ${name} is not writable` });
            }
        }
    }

    /** Decide whether the caller has a permission on one object of a call's type. */
    async #permits(
        call: Call,
        id: string,
        permission: access.PermissionReference,
    ): Promise<boolean> {
        const { object, scope } = call;
        if (object.storage !== "durable") {
            const granted = await this.in(
                this.authorizer.governingScope(object.reference(scope, id)),
            );

            return (await this.#decide(call, id, permission, granted)).length > 0;
        }

        return (await this.check(permission, object.reference(scope, id))).isAllowed;
    }

    /** Read an ephemeral or external object the access has a permission on, decided in memory. */
    async #decide(
        call: Call,
        id: string,
        permission: access.PermissionReference,
        evaluated: Access,
    ): Promise<Row[]> {
        // read the row in the call's scope: an event from the event log, any other through the transaction
        const row =
            call.object.storage !== "events"
                ? await call.requireTransaction().row(call.object.table, { id })
                : await EventListing.row(call, id);
        const rows: readonly Row[] = row === null || row["scope"] !== call.scope ? [] : [row];

        return this.keep(rows, permission, evaluated);
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
