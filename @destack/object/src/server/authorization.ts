import * as access from "@destack/access";
import { Replica, Scope, type ObjectReference, type ScopeLink } from "@destack/sync";
import {
    AccessError,
    earliest,
    AccessContext,
    type Authorizer,
    type GrantReader,
    type Access,
    type Permission,
    Caller,
} from "@destack/access";
import {
    and,
    eq,
    Key,
    ne,
    or,
    sql,
    TABLE,
    type DatabaseConnection,
    type SQL,
    Snapshot,
    type Row,
} from "@destack/db";
import { aligned, schema } from "@destack/schema";
import { conceal, ServiceError } from "@destack/service/error";
import { Moved } from "@destack/directory";
import type { Call } from "../method/call.ts";
import type { MethodKind } from "../method/kind.ts";
import { SCOPE_READ, type ObjectType } from "../object/object.ts";

/** The kinds of method a scope with capped storage still runs, since they free storage. */
const FREEING_KINDS: ReadonlySet<MethodKind> = new Set(["delete", "purge"]);

/** A caller's authorization in one call's transaction. */
export class Authorization extends access.Authorization {
    /** The caller's resolved access in the call's scope. */
    readonly access: Access;
    /** The token lending the caller's authority to the installation serving the call, for the calls it sends. */
    readonly delegation?: string;

    /** Authorize a caller in one transaction. */
    constructor(
        authorizer: Authorizer,
        database: DatabaseConnection,
        bind: (scope: string) => AccessContext,
        resolved: Access,
        delegation?: string,
    ) {
        super(authorizer, database, bind, resolved);
        this.access = resolved;
        if (delegation !== undefined) {
            this.delegation = delegation;
        }
    }

    /** Whether the system runs the call, admitted to every permission. */
    get isSystem(): boolean {
        return false;
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

    /**
     * Decide which rows the caller may list.
     *
     * The copies an enclosing scope hands down and the copies kept for the scope are admitted.
     * The scope's own rows are decided by the caller's access in it, and the rows of a scope it encloses by the caller's access there.
     */
    async admitRows(
        object: ObjectType,
        permission: Permission,
        scope: string,
        rows: readonly Row[],
        reader?: GrantReader,
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
                this.database,
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

    /** Read a call's target as a snapshot shows it, where the caller has a permission now and, at a position, then. */
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
                        await this.authorizer.resolve(snapshot, governing, this.context(governing)),
                    )
                ).isAllowed);

        // hide a missing row and a row the caller may not read
        if (row === undefined || !isAllowed) {
            throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${id}` });
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

        // challenge for step-up authentication first and for a lent delegate's missing grant second
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
        await this.#requireLentGrant(call, id, permission, governing, select);

        // refuse a reader, and hide the object from others
        const isReadable =
            object.reading !== undefined && (await this.#permits(call, id, object.reading));
        const denial = new ServiceError("FORBIDDEN", {
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
        // decide ephemeral and external objects in memory
        const { object, scope } = call;
        if (object.storage !== "durable") {
            return this.#decide(call, id, permission, evaluated);
        }

        // select a durable object where the permission holds
        const table = object.table;
        const target = object.reference(scope, id);

        return this.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table[TABLE].column("id"), id),
                    object.inScope(scope),
                    this.authorizer.permits(permission, target, evaluated),
                ),
            );
    }

    /** Refuse a target a lent delegate lacks the grant for, which the delegation before it reads. */
    async #requireLentGrant(
        call: Call,
        id: string,
        permission: Permission,
        governing: Access,
        select: (access: Access) => Promise<readonly Row[]>,
    ): Promise<void> {
        // walk the delegation chain
        const { object, scope } = call;
        const context = governing.context;
        const delegates = context.delegates ?? [];
        const chain = Caller.delegation(context);
        const governingScope = this.authorizer.governingScope(object.reference(scope, id));
        for (const [position, delegate] of delegates.entries()) {
            // skip a delegate acting with full authority
            if (delegate.authority === "full") {
                continue;
            }

            // compare the access before and after lending to the delegate
            const lending = await this.authorizer.resolve(this.snapshot, governingScope, {
                ...context,
                delegates: delegates.slice(0, position),
            });
            const lent = await this.authorizer.resolve(this.snapshot, governingScope, {
                ...context,
                delegates: delegates.slice(0, position + 1),
            });
            if ((await select(lending)).length > 0 && (await select(lent)).length === 0) {
                throw new ServiceError("INSUFFICIENT_GRANT", {
                    message: "invite the principal the delegate acts for to the missing delegation",
                    data: {
                        permission,
                        object: object.reference(scope, id),
                        delegate: aligned(chain, position).delegate,
                        onBehalfOf: aligned(chain, position).delegator,
                    },
                });
            }
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
            throw new ServiceError("NOT_FOUND", { message: `no scope ${scope}` });
        }

        // decide for the person when the credential's restrictions include the scope
        const { permissions, ...person } = this.access.context;
        const isNamed = permissions?.some((restriction) => restriction.scope === scope) === true;
        const context = isNamed ? person : this.access.context;
        const caller = (bound: string) =>
            isNamed ? this.authorizer.resolve(this.snapshot, bound, person) : this.in(bound);

        // decide for the caller and for the lending principal
        const delegates = context.delegates ?? [];
        const lent = delegates.findIndex((delegate) => delegate.authority === "lent");
        const principal = { ...context, delegates: delegates.slice(0, lent) };
        const isVisible =
            (await this.#sees(own, objects, caller)) ||
            (lent !== -1 &&
                (await this.#sees(own, objects, (bound) =>
                    this.authorizer.resolve(this.snapshot, bound, principal),
                )));
        if (!isVisible) {
            const denial = new ServiceError("FORBIDDEN", {
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
            const [row] = await this.database
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
            ? object.scope === Scope.universe.id
            : own?.id === this.access.scope && object.livesIn(own);
    }

    /** Refuse an object type living outside the admitted scope's type. */
    requireScopeOf(object: ObjectType): void {
        if (!this.isScopeOf(object)) {
            throw new ServiceError("NOT_FOUND", {
                message: `no ${object.plural} in scope ${this.access.scope}`,
            });
        }
    }

    /** Refuse a moved scope with its new holder. */
    requireUnmoved(): void {
        const moved = this.access.moved;
        if (moved !== undefined) {
            throw Moved.error(moved);
        }
    }

    /** Refuse calls writing to a scope whose storage is capped: every mutating call but a delete or a purge. */
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
            throw new ServiceError("QUOTA_EXCEEDED", {
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
                throw new ServiceError("FORBIDDEN", { message: `field ${name} is not writable` });
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
        // read the row
        const table = call.object.table;
        const rows: readonly Row[] = await call.database
            .select()
            .from(table)
            .where(and(eq(table[TABLE].column("id"), id), call.object.inScope(call.scope)));

        return this.keep(rows, permission, evaluated);
    }

    /** Keep the rows the caller also had a permission on at a snapshot's position. */
    async keepAt<Kept extends Row>(
        rows: readonly Kept[],
        permission: access.PermissionReference,
        snapshot: Snapshot,
        scope: string,
    ): Promise<Kept[]> {
        const then = await this.authorizer.resolve(snapshot, scope, this.context(scope));
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
        reader?: GrantReader,
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

    /** Omit from each row the guarded fields the caller may not read on it. */
    async redact(object: ObjectType, rows: readonly Row[]): Promise<Row[]> {
        // omit sensitive and unreadable fields
        const hidden =
            object.guarded.length === 0
                ? rows.map((): string[] => [])
                : (await this.concealed(object, rows)).hidden;

        return rows.map((row, position) =>
            omit(row, [...object.sensitive, ...aligned(hidden, position)]),
        );
    }
}

/** The system's authorization, admitted to every permission. */
export class SystemAuthorization extends Authorization {
    /** Authorize the system in one scope of a database at a time. */
    static async open(
        authorizer: Authorizer,
        database: DatabaseConnection,
        scope: string,
        now: number,
        links?: readonly ScopeLink[],
    ): Promise<SystemAuthorization> {
        const bind = (): AccessContext => ({ subjects: [], attributes: {}, now });
        const resolved = await authorizer.resolve(Snapshot.live(database), scope, bind(), links);

        return new SystemAuthorization(authorizer, database, bind, resolved);
    }

    /** Authorize the system within a transaction. */
    override within(transaction: DatabaseConnection): SystemAuthorization {
        return new SystemAuthorization(
            this.authorizer,
            transaction,
            (scope) => this.context(scope),
            this.access,
        );
    }

    /** Run the call as the system. */
    override get isSystem(): boolean {
        return true;
    }

    /** Admit every permission on every object. */
    override async check(): Promise<access.Decision> {
        return { isAllowed: true };
    }

    /** Admit every grant, including the relations only the system grants. */
    protected override async authorizeGrant(): Promise<void> {}

    /** Admit every revocation, including the relations only the system grants. */
    protected override async authorizeRevoke(): Promise<void> {}

    /** Admit every role change, including the roles a declaration manages. */
    protected override async authorizeRole(): Promise<void> {}

    /** Admit every permission on every row. */
    override async checkRows(
        _permission: access.PermissionReference,
        _scope: string,
        rows: readonly Row[],
    ): Promise<access.Admission> {
        return { permitted: new Set(rows.keys()) };
    }

    /** Match every row of an object type. */
    override listable(): SQL {
        return sql`true`;
    }

    /** Read one object of the call's scope and refuse a missing one. */
    override async read(call: Call, id: string): Promise<Row> {
        // read the row
        const table = call.object.table;
        const rows: readonly Row[] = await call.database
            .select()
            .from(table)
            .where(and(eq(table[TABLE].column("id"), id), call.object.inScope(call.scope)));
        const [row] = rows;
        if (row === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no ${call.object.name} ${id}` });
        }

        return row;
    }

    /** See every scope. */
    override async requireVisible(): Promise<void> {}

    /** Write every guarded field. */
    override async requireWritable(): Promise<void> {}

    /** Conceal no field. */
    override async concealed(
        _object: ObjectType,
        rows: readonly Row[],
    ): Promise<{ readonly hidden: string[][] }> {
        return { hidden: rows.map(() => []) };
    }
}

/** Copy a row without some of its fields. */
function omit(row: Row, names: readonly string[]): Row {
    return names.length === 0
        ? row
        : Object.fromEntries(Object.entries(row).filter(([name]) => !names.includes(name)));
}
