import * as access from "@destack/access";
import {
    AccessError,
    GLOBAL_SCOPE,
    delegationChain,
    earliest,
    type AccessContext,
    type Authorizer,
    type GrantReader,
    type Access,
    type Permission,
} from "@destack/access";
import { and, eq, or, sql, type DatabaseConnection, type SQL, type Table } from "@destack/db";
import { Snapshot } from "@destack/db/log";
import { ServiceError } from "@destack/service/error";
import { ScopeHolder } from "../error/holder.ts";
import type { Call } from "../method/call.ts";
import { SCOPE_READ, type ObjectType } from "../object/object.ts";

/** A caller's authorization in one call's transaction. */
export class Authorization extends access.Authorization {
    /** The caller's resolved access in the call's scope. */
    readonly access: Access;

    /** Authorize a caller in one transaction. */
    constructor(
        authorizer: Authorizer,
        database: DatabaseConnection,
        bind: (scope: string) => AccessContext,
        resolved: Access,
    ) {
        super(authorizer, database, bind, resolved);
        this.access = resolved;
    }

    /** Match the rows the caller may list. */
    listable(object: ObjectType, permission: Permission): SQL | "memory" {
        return object.storage === "ephemeral"
            ? "memory"
            : this.authorizer.where(permission, this.access, object.table as Table);
    }

    /** Read a call's target where the caller holds the method's permission. */
    async read(call: Call, id: string): Promise<Record<string, unknown>> {
        // read where the caller holds the permission
        const { object, scope } = call;
        const permission = object.permission(call.method.permission!);
        const table = object.table as Table & Record<string, never>;
        const target = object.reference(scope, id);
        const select = async (evaluated: Access) =>
            object.storage === "ephemeral"
                ? await this.#decide(call, id, permission, evaluated)
                : ((await this.database
                      .select()
                      .from(table)
                      .where(
                          and(
                              eq(table.id, id),
                              object.inScope(scope),
                              this.authorizer.holds(permission, target, evaluated),
                          ),
                      )) as Record<string, unknown>[]);
        const governing = await this.in(this.authorizer.governingScope(target));
        const [row] = await select(governing);
        if (row) {
            return row;
        }

        // challenge for step-up authentication
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

        // challenge a lent delegate for a missing grant
        const context = governing.context;
        const delegates = context.delegates ?? [];
        const chain = delegationChain(context);
        for (let position = 0; position < delegates.length; position++) {
            if (delegates[position]!.authority === "full") {
                continue;
            }
            const lending = await this.authorizer.resolve(
                this.snapshot,
                this.authorizer.governingScope(target),
                {
                    ...context,
                    delegates: delegates.slice(0, position),
                },
            );
            const lent = await this.authorizer.resolve(
                this.snapshot,
                this.authorizer.governingScope(target),
                {
                    ...context,
                    delegates: delegates.slice(0, position + 1),
                },
            );
            if ((await select(lending)).length > 0 && (await select(lent)).length === 0) {
                throw new ServiceError("INSUFFICIENT_GRANT", {
                    status: 403,
                    message:
                        "propose the missing delegation to the principal the delegate acts for",
                    data: {
                        permission,
                        object: object.reference(scope, id),
                        delegate: chain[position]!.delegate,
                        onBehalfOf: chain[position]!.delegator,
                    },
                });
            }
        }

        // check whether the caller reads the object
        const isReadable =
            object.reading !== undefined && (await this.#holds(call, id, object.reading));

        // refuse a reader
        if (isReadable) {
            throw new ServiceError("FORBIDDEN", {
                message: `permission denied: ${permission.name}`,
            });
        }
        // hide the object from others
        else {
            throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${id}` });
        }
    }

    /** Require the call's scope to be visible to the caller. */
    async requireVisible(objects: readonly ObjectType[]): Promise<void> {
        // see the global scope
        const scope = this.access.scope;
        if (scope === GLOBAL_SCOPE) {
            return;
        }

        // find the scope's own type
        const own = this.access.scopes[0];
        const type =
            own?.id === scope
                ? objects
                      .flatMap((object) => object.scopes)
                      .find((candidate) => candidate.policy.is(own))
                : undefined;
        if (type === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no scope ${scope}` });
        }

        // decide for the person when the credential's restrictions name the scope, which its issuer revealed
        const { permissions, ...person } = this.access.context;
        const isNamed = permissions?.some((restriction) => restriction.scope === scope) === true;
        const context = isNamed ? person : this.access.context;
        const caller = (bound: string) =>
            isNamed ? this.authorizer.resolve(this.snapshot, bound, person) : this.in(bound);

        // decide for the caller, then for the lending principal
        const delegates = context.delegates ?? [];
        const lent = delegates.findIndex((delegate) => delegate.authority === "lent");
        const principal = { ...context, delegates: delegates.slice(0, lent) };
        const isVisible =
            (await this.#sees(type, own!, objects, caller)) ||
            (lent !== -1 &&
                (await this.#sees(type, own!, objects, (bound) =>
                    this.authorizer.resolve(this.snapshot, bound, principal),
                )));
        if (!isVisible) {
            throw new ServiceError("NOT_FOUND", { message: `no scope ${scope}` });
        }
    }

    /** Decide whether an access sees a scope. */
    async #sees(
        type: ObjectType,
        own: access.ObjectReference,
        objects: readonly ObjectType[],
        resolve: (scope: string) => Promise<Access>,
    ): Promise<boolean> {
        // see a readable scope
        const container = await resolve(this.authorizer.governingScope(own));
        const permission = type.permission(SCOPE_READ);
        if ((await this.authorizer.check(this.snapshot, permission, own, container)).isAllowed) {
            return true;
        }

        // see a scope holding a permitted durable object
        const scope = own.id;
        const evaluated = await resolve(scope);
        const inside = objects.filter(
            (object) => object.storage === "durable" && object.scopes.includes(type),
        );
        for (const object of inside) {
            const table = object.table as Table & Record<string, never>;
            const held = object.permissions.map((name) =>
                this.authorizer.where(object.permission(name), evaluated, table),
            );
            const [row] = await this.database
                .select({ id: table.id })
                .from(table)
                .where(and(object.inScope(scope), or(...held)))
                .limit(1);
            if (row !== undefined) {
                return true;
            }
        }

        return false;
    }

    /** Refuse an object type living outside the admitted scope's type. */
    requireScopeOf(object: ObjectType): void {
        // require the object type's home scope
        const own = this.access.scopes[0];
        const isHome =
            this.access.scope === GLOBAL_SCOPE
                ? object.scopes.length === 0
                : own?.id === this.access.scope &&
                  object.scopes.some((type) => type.policy.is(own));
        if (!isHome) {
            throw new ServiceError("NOT_FOUND", {
                message: `no ${object.plural} in scope ${this.access.scope}`,
            });
        }
    }

    /** Refuse a moved scope, naming its new holder. */
    requireUnmoved(): void {
        const moved = this.access.moved;
        if (moved !== undefined) {
            throw ScopeHolder.error(moved);
        }
    }

    /** Require the write permission of every guarded field a call sets. */
    async requireWritable(call: Call, id: string): Promise<void> {
        for (const [name, declared] of Object.entries(call.object.fields)) {
            const permission = declared.access?.write;
            if (
                permission !== undefined &&
                Object.hasOwn(call.input, name) &&
                !(await this.#holds(call, id, call.object.permission(permission)))
            ) {
                throw new ServiceError("FORBIDDEN", { message: `field ${name} is not writable` });
            }
        }
    }

    /** Decide whether the caller holds a permission on one object of a call's type. */
    async #holds(call: Call, id: string, permission: access.PermissionReference): Promise<boolean> {
        const { object, scope } = call;
        if (object.storage === "ephemeral") {
            const access = await this.in(
                this.authorizer.governingScope(object.reference(scope, id)),
            );

            return (await this.#decide(call, id, permission, access)).length > 0;
        }

        return (await this.check(permission, object.reference(scope, id))).isAllowed;
    }

    /** Read an ephemeral object the access holds a permission on, decided in memory. */
    async #decide(
        call: Call,
        id: string,
        permission: access.PermissionReference,
        evaluated: Access,
    ): Promise<Record<string, unknown>[]> {
        // read the row
        const table = call.object.table as Table & Record<string, never>;
        const rows = (await call.database
            .select()
            .from(table)
            .where(and(eq(table.id, id), call.object.inScope(call.scope)))) as Record<
            string,
            unknown
        >[];

        return this.keep(rows, permission, evaluated);
    }

    /** Keep the rows an access holds a permission on, decided in memory. */
    async keep<Row extends Record<string, unknown>>(
        rows: readonly Row[],
        permission: access.PermissionReference,
        evaluated: Access = this.access,
    ): Promise<Row[]> {
        const { held } = await this.authorizer.checkRows(
            this.snapshot,
            permission,
            evaluated,
            rows,
        );

        return rows.filter((_, position) => held.has(position));
    }

    /** List each row's guarded fields the caller may not read, and until when. */
    async concealed(
        object: ObjectType,
        rows: readonly Readonly<Record<string, unknown>>[],
        reader?: GrantReader,
    ): Promise<{ readonly hidden: string[][]; readonly until?: number }> {
        // group the guarded fields by read permission
        const hidden = rows.map((): string[] => []);
        const permissions = new Map<string, string[]>();
        for (const name of object.guarded) {
            const permission = object.fields[name]!.access!.read!;
            permissions.set(permission, [...(permissions.get(permission) ?? []), name]);
        }

        // check each distinct read permission once over every row
        const moments: (number | undefined)[] = [];
        for (const [permission, names] of permissions) {
            const readable = await this.checkRows(
                object.permission(permission),
                this.access.scope,
                rows,
                reader,
            );
            moments.push(readable.until);
            for (const [position, fields] of hidden.entries()) {
                if (!readable.held.has(position)) {
                    fields.push(...names);
                }
            }
        }
        const until = earliest(moments);

        return { hidden, ...(until === undefined ? {} : { until }) };
    }

    /** Omit from each row the guarded fields the caller may not read on it. */
    async redact<Row extends Readonly<Record<string, unknown>>>(
        object: ObjectType,
        rows: readonly Row[],
    ): Promise<Row[]> {
        // omit sensitive and unreadable fields
        const hidden =
            object.guarded.length === 0
                ? rows.map((): string[] => [])
                : (await this.concealed(object, rows)).hidden;

        return rows.map((row, position) => omit(row, [...object.sensitive, ...hidden[position]!]));
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
    ): Promise<SystemAuthorization> {
        const bind = (): AccessContext => ({ subjects: [], attributes: {}, now });
        const resolved = await authorizer.resolve(Snapshot.live(database), scope, bind());

        return new SystemAuthorization(authorizer, database, bind, resolved);
    }

    /** Admit every permission on every object. */
    override async check(): Promise<access.Decision> {
        return { isAllowed: true };
    }

    /** Admit every permission on every row. */
    override async checkRows(
        _permission: access.PermissionReference,
        _scope: string,
        rows: readonly Readonly<Record<string, unknown>>[],
    ): Promise<access.Admission> {
        return { held: new Set(rows.keys()) };
    }

    /** Match every row of an object type. */
    override listable(): SQL {
        return sql`true`;
    }

    /** Read one object of the call's scope, refusing one that does not exist. */
    override async read(call: Call, id: string): Promise<Record<string, unknown>> {
        // read the row
        const table = call.object.table as Table & Record<string, never>;
        const [row] = (await call.database
            .select()
            .from(table)
            .where(and(eq(table.id, id), call.object.inScope(call.scope)))) as Record<
            string,
            unknown
        >[];
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
        rows: readonly Readonly<Record<string, unknown>>[],
    ): Promise<{ readonly hidden: string[][] }> {
        return { hidden: rows.map(() => []) };
    }
}

/** Copy a row without some of its fields. */
function omit<Row extends Readonly<Record<string, unknown>>>(
    row: Row,
    names: readonly string[],
): Row {
    return names.length === 0
        ? row
        : (Object.fromEntries(
              Object.entries(row).filter(([name]) => !names.includes(name)),
          ) as Row);
}
