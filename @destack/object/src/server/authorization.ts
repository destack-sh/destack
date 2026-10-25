import type { Environment } from "@destack/environment";
import type { Manager } from "@destack/access";
import { ObjectError } from "../error/error.ts";
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
import { schema, zip } from "@destack/schema";
import { conceal, CallError } from "../error/call.ts";
import type { MethodContext } from "../method/context.ts";
import { EventLog } from "../query/event.ts";
import type { StoreExecution } from "../method/execution.ts";
import type { MethodKind } from "../method/kind.ts";
import { SCOPE_LIST, SCOPE_READ, type ObjectType } from "../object/object.ts";

import type {} from "@destack/package/import-meta";

/** How a decision treats replicas of other scopes: passed as included, or checked as local rows. */
type ReplicaDecision = "included" | "checked";

/** The kinds of method a scope with capped storage still runs, since they free storage. */
const FREEING_KINDS: ReadonlySet<MethodKind> = new Set(["delete", "deleteMany", "purge"]);

/** A caller's authorization in one call's transaction. */
export class Authorization extends access.Authorization {
    /** The caller's resolved access in the call's scope. */
    readonly access: Access;
    /** The transaction's writes to the deciding tables when the access was resolved. */
    readonly #writes: number;

    /** Authorize a caller in one transaction. */
    constructor(
        authorizer: Authorizer,
        transaction: Transaction,
        context: () => AccessContext,
        environment: Pick<Environment, "clock" | "crypto" | "random">,
        options: {
            /** The caller's access in the call's scope. */
            readonly resolved: Access;
            /** The applied declaration writing the rows. */
            readonly manager?: Manager;
        },
    ) {
        super(authorizer, transaction, context, environment, options);
        this.access = options.resolved;
        this.#writes = transaction.changes.writes(authorizer.decidingTables);
    }

    /** Authorize the caller to apply a declaration that manages its rows, keeping its access. */
    override applying(manager: Manager): Authorization {
        // read the caller as it acts now, as the copied authorization does
        const context = () => this.context();

        return new Authorization(this.authorizer, this.transaction, context, this.environment, {
            resolved: this.access,
            manager,
        });
    }

    /** Report whether the transaction wrote nothing the access was resolved from since. */
    get isCurrent(): boolean {
        return this.transaction.changes.writes(this.authorizer.decidingTables) === this.#writes;
    }

    /** Match the rows the caller has a permission on, and a required one too, or "memory". */
    where(object: ObjectType, permission: Permission, required?: Permission): SQL | "memory" {
        // match both permissions, checking the required one on replicas too
        const permitted = this.#where(object, permission, "included");
        const held = required === undefined ? undefined : this.#where(object, required, "checked");
        if (permitted === "memory" || held === "memory") {
            return "memory";
        }

        return held === undefined ? permitted : and(permitted, held);
    }

    /** Filter the rows in hand the caller has a permission on, and a required one too. */
    async filter<Filtered extends Row>(
        object: ObjectType,
        permission: Permission,
        rows: readonly Filtered[],
        options: {
            readonly scope?: string;
            readonly reader?: TupleReader;
            readonly required?: Permission;
        } = {},
    ): Promise<{
        readonly rows: Filtered[];
        readonly until?: number;
        readonly below: readonly string[];
    }> {
        // filter by the permission, then by the required one checked on replicas too
        const { required, ...deciding } = options;
        const permitted = await this.#filter(object, permission, rows, deciding, "included");
        if (required === undefined) {
            return permitted;
        }
        const held = await this.#filter(object, required, permitted.rows, deciding, "checked");
        const until = earliest([permitted.until, held.until]);

        return {
            rows: held.rows,
            ...(until === undefined ? {} : { until }),
            below: [...new Set([...permitted.below, ...held.below])],
        };
    }

    /** Match the rows the caller has a permission on in SQL, or "memory" for `#filter`. */
    #where(object: ObjectType, permission: Permission, replicas: ReplicaDecision): SQL | "memory" {
        // decide ephemeral and external rows in memory, and checked replicas at their scopes
        if (object.storage !== "durable" || replicas === "checked") {
            return "memory";
        }

        // match the permitted rows
        const table = object.table;
        const permitted = this.authorizer.where(permission, this.access, table);
        const isReplica = sql<boolean>`${table[TABLE].column("scope")} <> ${this.access.scope}`;

        // pass the replicas handed down from enclosing scopes
        if (object.inherited !== undefined) {
            return or(isReplica, permitted);
        }
        // pass the replicas decided for the caller below the scope
        else {
            const caller = Caller.principal(this.context());

            return or(
                and(isReplica, Replica.includes(table, this.access.scope, caller)),
                permitted,
            );
        }
    }

    /** Decide which rows in hand the caller has a permission on, as `#where` matches them. */
    async #filter<Filtered extends Row>(
        object: ObjectType,
        permission: Permission,
        rows: readonly Filtered[],
        options: { readonly scope?: string; readonly reader?: TupleReader },
        replicas: ReplicaDecision,
    ): Promise<{
        readonly rows: Filtered[];
        readonly until?: number;
        readonly below: readonly string[];
    }> {
        // decide in the call's scope
        const { scope = this.access.scope, reader } = options;

        // place each row in its scope
        const table = object.table;
        const scoped = rows.map((row, position) => ({
            position,
            row,
            scope: schema.string().parse(row["scope"]),
        }));
        const others = scoped.filter((entry) => entry.scope !== scope);
        let passed: ReadonlySet<number>;
        let below: ReadonlyMap<string, Access> = new Map();
        // check replicas like local rows
        if (replicas === "checked") {
            passed = new Set();
        }
        // pass the replicas handed down from enclosing scopes wholesale
        else if (object.inherited !== undefined) {
            passed = new Set(others.map((entry) => entry.position));
        }
        // pass the durable replicas the caller's replica includes
        else if (object.storage === "durable" && others.length > 0) {
            const included = await Replica.keysIncluded(
                this.transaction.database,
                table,
                others.map((entry) => entry.row),
                scope,
                Caller.principal(this.context()),
            );
            passed = new Set(
                others
                    .filter((entry) => included.has(Key.name(table, entry.row)))
                    .map((entry) => entry.position),
            );
        }
        // pass no other scope's rows
        else {
            passed = new Set();
        }

        // resolve the scopes the scope encloses, which the rows left over live in
        const left = others.filter((entry) => !passed.has(entry.position));
        if (object.storage === "durable" && object.inherited === undefined && left.length > 0) {
            below = await this.descend(scope, [...new Set(left.map((entry) => entry.scope))]);
        }

        // check the permission on the scope's rows and on the rows of the scopes it encloses
        const local = scoped.filter(
            (entry) =>
                !passed.has(entry.position) && (entry.scope === scope || below.has(entry.scope)),
        );
        const checked = await this.checkRows(
            permission,
            scope,
            local.map((entry) => entry.row),
            reader,
            below,
        );

        // check the permission on each checked replica of another scope at its governing scope
        const outside = left.filter((entry) => replicas === "checked" && !below.has(entry.scope));
        const decisions = await Promise.all(
            outside.map(async (entry) => {
                const id = schema.string().parse(entry.row["id"]);
                const decision = await this.check(permission, object.reference(entry.scope, id));

                return { entry, decision };
            }),
        );
        const until = earliest([checked.until, ...decisions.map(({ decision }) => decision.until)]);

        // permit the passed replicas, the permitted local rows and the allowed checked replicas
        const permitted = new Set([
            ...passed,
            ...local
                .filter((_entry, index) => checked.permitted.has(index))
                .map((entry) => entry.position),
            ...decisions
                .filter(({ decision }) => decision.isAllowed)
                .map(({ entry }) => entry.position),
        ]);

        return {
            rows: rows.filter((_row, position) => permitted.has(position)),
            ...(until === undefined ? {} : { until }),
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
        // read the row the view shows, allowed with the caller's grants now
        const { object, scope } = context;
        const row = await snapshot.read(object.table).findFirst({ where: { id } });
        const isVisible =
            row !== undefined &&
            (await this.filter(object, permission, [row], { scope })).rows.length > 0;

        // decide the permission as of a position too
        const reference = object.reference(scope, id);
        const governing = this.authorizer.governingScope(reference);
        const isAllowed =
            isVisible &&
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
            throw CallError.http("NOT_FOUND", {
                message: `${object.name} ${id} not found`,
            });
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
            throw AccessError.of(
                "AUTHENTICATION_INSUFFICIENT",
                "authentication assurance is too low: authenticate again",
                { data: stepUp },
            );
        }
        await this.#requireGranted(context, id, permission, governing, select);

        // refuse a reader, and hide the object from others
        const isReadable =
            object.reading !== undefined &&
            (await this.#permits(context, id, object.reading, execution));
        const denial = ObjectError.of("PERMISSION_DENIED", `permission denied: ${permission.name}`);
        throw isReadable ? denial : conceal(denial, `${object.name} ${id} not found`);
    }

    /** Select a call's target where an access has a permission on it. */
    async #selectPermitted(
        context: MethodContext,
        id: string,
        permission: Permission,
        evaluated: Access,
        execution: StoreExecution,
    ): Promise<readonly Row[]> {
        // read the object through the call's transaction, its writes included, and decide in memory
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
        const { grants } = governing.context;
        const grant = grants.at(-1);
        if (grant === undefined) {
            return;
        }
        const subject = { ...governing.context, actors: [], grants: [] };

        // ask for approval where the grant gives the permission once approved, else refuse the grant
        const governingScope = this.authorizer.governingScope(object.reference(scope, id));
        const alone = await this.authorizer.resolve(this.snapshot, governingScope, subject);
        const isSubjects = (await select(alone)).length > 0;
        const permissions = governing.permissions;
        if (
            isSubjects &&
            access.GrantPermission.allowsOnApproval(permission, { scope, id }, permissions)
        ) {
            throw ObjectError.of(
                "APPROVAL_REQUIRED",
                `grant ${grant.id} gives ${permission.name} on ${object.name} ${id} only after approval`,
                { data: { permission, object: object.reference(scope, id), grant: grant.id } },
            );
        } else if (isSubjects) {
            throw ObjectError.of(
                "GRANT_INSUFFICIENT",
                `grant ${grant.id} does not give ${permission.name} on ${object.name} ${id}`,
                { data: { permission, object: object.reference(scope, id), grant: grant.id } },
            );
        }
    }

    /** Hide a refusal in a scope the caller cannot see behind the scope's absence. */
    async concealDenial(error: unknown, objects: readonly ObjectType[]): Promise<unknown> {
        // pass on anything but a missing or hidden object and a raw access denial
        const isMissing = CallError.of(error)?.code === "NOT_FOUND";
        const isRefused = access.AccessError.is(error) && error.code === "FORBIDDEN";
        if (!(isMissing || isRefused) || (await this.sees(objects))) {
            return error;
        }

        return ObjectError.of("SCOPE_NOT_FOUND", `scope ${this.access.scope} not found`, {
            cause: error,
        });
    }

    /** Refuse a caller listing in a scope it cannot see, as the scope's absence. */
    async requireSeen(objects: readonly ObjectType[]): Promise<void> {
        if (!(await this.sees(objects))) {
            throw ObjectError.of("SCOPE_NOT_FOUND", `scope ${this.access.scope} not found`);
        }
    }

    /** Decide whether the caller sees the authorized scope, as the universe everyone sees. */
    async sees(objects: readonly ObjectType[]): Promise<boolean> {
        // see the universe
        const scope = this.access.scope;
        if (scope === Scope.universe.id) {
            return true;
        }

        // decide for the caller, or only for its subject where its grant gives something in the scope
        const first = this.access.scopes[0];
        const permissions = this.access.permissions;
        const subject = { ...this.access.context, actors: [], grants: [] };
        const isScope = first?.id === scope && objects.some((object) => object.livesIn(first));

        return (
            isScope &&
            (permissions === undefined
                ? await this.#sees(first, (bound) => this.in(bound))
                : permissions.some((entry) =>
                      access.GrantPermission.overlaps(entry.scope, scope),
                  ) &&
                  (await this.#sees(first, (bound) =>
                      this.authorizer.resolve(this.snapshot, bound, subject),
                  )))
        );
    }

    /** Decide whether an access sees a scope: it reads or lists it, or has a grant inside it. */
    async #sees(
        reference: ObjectReference,
        resolve: (scope: string) => Promise<Access>,
    ): Promise<boolean> {
        // see a scope the access reads or lists, by the permissions its type declares
        const container = await resolve(this.authorizer.governingScope(reference));
        const declared = this.authorizer.policy(reference).definition.permissions;
        const names = [SCOPE_READ, SCOPE_LIST].filter((name) => Object.hasOwn(declared, name));
        for (const name of names) {
            const permission = { packageId: reference.packageId, type: reference.type, name };
            const decided = await this.authorizer.check(
                this.snapshot,
                permission,
                reference,
                container,
            );
            if (decided.isAllowed) {
                return true;
            }
        }

        // see a scope the access has a granted relationship inside, such as a shared object
        return this.authorizer.isRelatedIn(
            this.snapshot,
            reference.id,
            await resolve(reference.id),
        );
    }

    /** Decide whether an object type's rows live in scopes of the authorized scope's type. */
    isScopeOf(object: ObjectType): boolean {
        const first = this.access.scopes[0];

        return this.access.scope === Scope.universe.id
            ? object.isInUniverse
            : first?.id === this.access.scope && object.livesIn(first);
    }

    /** Decide whether the authorized scope has every row of a type its replicas read. */
    isHomeOf(object: ObjectType): boolean {
        return this.access.scope === Scope.universe.id
            ? object.scope === Scope.universe.id
            : this.isScopeOf(object);
    }

    /** Refuse a type routed outside the authorized scope. */
    requireScopeOf(object: ObjectType): void {
        if (!this.isScopeOf(object)) {
            throw ObjectError.of(
                "TYPE_NOT_IN_SCOPE",
                `no ${object.plural} in scope ${this.access.scope}`,
            );
        }
    }

    /** Refuse work this store no longer or never serves: a moved scope, or another partition. */
    requireHere(
        permissions: readonly access.PermissionReference[],
        served: string | undefined,
        accessed: string | undefined,
    ): void {
        this.#requireUnmoved(permissions);
        this.#requirePartition(served, accessed);
    }

    /** Refuse a moved scope with its new holder, unless every permission is a moving one. */
    #requireUnmoved(permissions: readonly access.PermissionReference[]): void {
        const moved = this.access.moved;
        const isMoving =
            permissions.length > 0 &&
            permissions.every((permission) =>
                this.authorizer.moving.has(access.PermissionReference.key(permission)),
            );
        if (moved !== undefined && !isMoving) {
            throw ObjectError.of("SCOPE_MOVED", `${moved.scope} moved to ${moved.machine}`, {
                data: moved,
            });
        }
    }

    /** Refuse a call in another partition than the store's, or in one inside the store's scope. */
    #requirePartition(served: string | undefined, accessed: string | undefined): void {
        // refuse another partition than the one served, or a partition nearer than the store's scope
        const { scope, partition, scopes } = this.access;
        const isOther = served !== undefined && partition !== served;
        const nearest = scopes.findIndex((link) => link.id === partition);
        const accessedAt =
            accessed === undefined ? -1 : scopes.findIndex((link) => link.id === accessed);
        const isInside =
            served === undefined && nearest !== -1 && accessedAt !== -1 && nearest < accessedAt;
        if (partition !== undefined && (isOther || isInside)) {
            throw ObjectError.of(
                "PARTITION_MISDIRECTED",
                `${scope} is served by partition ${partition}`,
                { data: { scope, partition } },
            );
        }
    }

    /** Refuse a method the caller's grants bind to other methods of its type. */
    requireMethods(calls: readonly { readonly object: ObjectType; readonly name: string }[]): void {
        // pass a caller under no grant
        const grant = this.access.context.grants.at(-1);
        if (grant === undefined) {
            return;
        }

        // refuse the first method a grant entry on its type leaves out
        const { scope, permissions } = this.access;
        const refused = calls.find(({ object, name }) => {
            const type = {
                packageId: object.policy.definition.packageId,
                type: object.policy.name,
            };

            return !access.GrantPermission.allowsMethod(type, scope, name, permissions);
        });
        if (refused !== undefined) {
            throw ObjectError.of(
                "GRANT_REFUSES_METHOD",
                `grant ${grant.id} does not allow ${refused.object.name}.${refused.name} in ${scope}`,
            );
        }
    }

    /** Refuse a call the caller's grants do not give, for a home that checks only the subject. */
    requireGranted(call: {
        readonly object: ObjectType;
        readonly name: string;
        readonly id?: string;
    }): void {
        // require the grants to allow the method
        this.requireMethods([call]);
        const grant = this.access.context.grants.at(-1);
        const { permission } = call.object.method(call.name);
        if (grant === undefined || permission === undefined) {
            return;
        }

        // require them to give its permission on the target
        const { scope, permissions } = this.access;
        const target = { scope, ...(call.id === undefined ? {} : { id: call.id }) };
        if (
            !access.GrantPermission.allows(
                call.object.policy.permission(permission),
                target,
                permissions,
            )
        ) {
            throw ObjectError.of(
                "GRANT_REFUSES_PERMISSION",
                `grant ${grant.id} does not give ${call.object.name}.${permission} in ${scope}`,
            );
        }
    }

    /** Refuse writes to a scope with capped storage: every mutating call but delete or purge. */
    requireUncapped(
        calls: readonly { readonly object: ObjectType; readonly name: string }[],
    ): void {
        // allow any call in a scope within its storage
        if (!this.access.isCapped) {
            return;
        }

        // refuse the first call that writes without freeing storage
        const writing = calls.find(({ object, name }) => {
            const { kind, mutates } = object.method(name);

            return mutates && !FREEING_KINDS.has(kind);
        });
        if (writing !== undefined) {
            throw ObjectError.of(
                "STORAGE_CAPPED",
                `storage of scope ${this.access.scope} is capped: ${writing.object.name}.${writing.name} is refused until data is deleted or the plan changes`,
            );
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
                throw ObjectError.of("FIELD_NOT_WRITABLE", `field ${name} is not writable`);
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

    /** Filter the rows the caller also had a permission on at a snapshot's position. */
    async filterAt<Filtered extends Row>(
        rows: readonly Filtered[],
        permission: access.PermissionReference,
        snapshot: Snapshot,
        scope: string,
    ): Promise<Filtered[]> {
        const then = await this.authorizer.resolve(snapshot, scope, this.context());
        const { permitted } = await this.authorizer.checkRows(snapshot, permission, then, rows);

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

        // decide the rows of the scopes below in their chains
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
            rows: zip(rows, hidden).map(([row, fields]) =>
                table.conceal(row, [...object.sensitive, ...fields]),
            ),
            isRevealing: object.reveals(rows, hidden),
        };
    }
}
