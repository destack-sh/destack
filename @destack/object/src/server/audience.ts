import {
    type ObjectReference,
    type RowKey,
    type Audience,
    type Watch,
    replica,
} from "@destack/sync";
import { accessRelationship, earliest, type Permission, type Subject } from "@destack/access";
import { and, eq, gt, or, sql, TABLE, type Row, type SQL, type Table } from "@destack/db";
import type { Change, LogPosition } from "@destack/db/log";
import { schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import type { ServiceContext } from "@destack/service/server";
import type { ObjectStorage, ObjectType } from "../object/object.ts";
import { ListedObjects } from "../query/listing.ts";
import type { Authorization } from "./authorization.ts";
import type { ObjectServer } from "./server.ts";

/** The most identifiers one dependents read lists, well within the parameter budget. */
const DEPENDENT_IDS = 500;

/** The columns naming a row's parent. */
const PARENT_COLUMNS = ["parentPackageId", "parentType", "parentId"] as const;

/** The caller's access to one stream of a scope's objects. */
export class ObjectAudience implements Audience {
    /** The server serving the objects. */
    readonly #server: Omit<ObjectServer, "router">;
    /** The scope the stream follows. */
    readonly #scope: string;
    /** Admit the subscriber again, as it resolves access once access changes. */
    readonly #admit: () => Promise<Authorization>;
    /** The object types by table. */
    readonly #objects: ListedObjects;
    /** The caller's access, resolved again once access changes. */
    #authorization: Authorization;
    /** The key of the caller's access inputs. */
    #key: string | undefined;
    /** The earliest moment time alone changes the caller's access. */
    #until: number | undefined;
    /** Where the followed objects live. */
    readonly #storage: ObjectStorage;
    /** The durable log position access was decided at, for ephemeral objects. */
    #decided: LogPosition | undefined;
    /** The scope chain's access rows, none for ephemeral objects. */
    watches: readonly Watch[];

    /** Serve a caller of a scope with resolved access. */
    private constructor(
        server: Omit<ObjectServer, "router">,
        scope: string,
        admit: () => Promise<Authorization>,
        authorization: Authorization,
        storage: ObjectStorage,
        decided: LogPosition | undefined,
    ) {
        // keep the inputs and watch access
        this.#server = server;
        this.#scope = scope;
        this.#admit = admit;
        this.#objects = new ListedObjects(server.objects);
        this.#authorization = authorization;
        this.#until = authorization.access.until;
        this.#storage = storage;
        this.#decided = decided;
        this.watches = storage === "durable" ? server.authorizer.watch(this.chain) : [];
    }

    /** Resolve a caller's access to a scope's objects. */
    static async open(
        server: Omit<ObjectServer, "router">,
        scope: string,
        context: ServiceContext,
        storage: ObjectStorage = "durable",
    ): Promise<ObjectAudience> {
        // decide ephemeral objects at the durable position
        const decided = storage === "ephemeral" ? await server.database.log.position() : undefined;

        // admit the follower
        const admit = () => server.admit(server.database, scope, context);

        return new ObjectAudience(server, scope, admit, await admit(), storage, decided);
    }

    /** Resolve a principal's access to a scope's durable objects, as the copies kept for it are decided. */
    static async of(
        server: Omit<ObjectServer, "router">,
        scope: string,
        subject: Subject,
    ): Promise<ObjectAudience> {
        const admit = () => server.authorizeSubject(server.database, scope, subject);

        return new ObjectAudience(server, scope, admit, await admit(), "durable", undefined);
    }

    /** The scope's own object, absent for a scope the database does not know. */
    get scope(): ObjectReference | undefined {
        return this.#authorization.access.scopes[0];
    }

    /** Match the rows of a table the caller may list. */
    where(table: Table): SQL | "memory" {
        const listed = this.#listed(table);

        return listed === "query"
            ? sql`true`
            : this.#authorization.listable(listed.object, listed.permission);
    }

    /** The key of the caller's access inputs. */
    get key(): string {
        this.#key ??= canonicalize({
            scope: this.#scope,
            context: { ...this.#authorization.access.context, request: null, now: null },
        });

        return this.#key;
    }

    /** Decide which rows the caller may list. */
    async admits(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<ReadonlySet<number>> {
        // admit every row left to its query
        const listed = this.#listed(table);
        if (listed === "query") {
            return new Set(rows.keys());
        }
        const { permission } = listed;

        // check the listing permission, admitting inherited copies
        const admission = await this.#authorization.admitRows(
            listed.object,
            permission,
            this.#scope,
            rows,
            this.#reader(position),
        );
        this.#until = earliest([this.#until, admission.until]);

        return admission.held;
    }

    /** List an object type's guarded fields. */
    concealable(table: Table): readonly string[] {
        return this.#objects.concealable(table);
    }

    /** List the guarded fields the caller may not read on each row. */
    async conceals(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<readonly (readonly string[])[]> {
        // conceal nothing of the rows left to their query
        const listed = this.#listed(table);
        if (listed === "query") {
            return rows.map(() => []);
        }
        const { object } = listed;

        // conceal unreadable fields
        const concealed = await this.#authorization.concealed(object, rows, this.#reader(position));
        this.#until = earliest([this.#until, concealed.until]);

        return concealed.hidden;
    }

    /** Read the earliest moment time alone changes the caller's access. */
    async until(): Promise<number | undefined> {
        return this.#until;
    }

    /** Resolve the caller's access again. */
    async refresh(): Promise<void> {
        await this.#authorize();
    }

    /** List the rows with access a change decides, or everything. */
    async dependents(change: Change): Promise<readonly RowKey[] | "everything"> {
        // resolve access after an access change
        if (
            this.#objects.get(change.table) === undefined &&
            change.table !== this.#server.journal.table
        ) {
            await this.#authorize();
            if (change.table !== accessRelationship) {
                return "everything";
            }
            const row = (change.before ?? change.after) as Row;
            const object = this.#server.objects.find(
                (entry) =>
                    entry.policy.definition.packageId === row.packageId &&
                    entry.policy.definition.name === row.type,
            );
            if (object === undefined) {
                return "everything";
            }

            return this.#dependentsOf(object, identifier(row.objectId));
        }

        // follow a moved row
        const object = this.#objects.get(change.table);
        const before = change.before as Row;
        const after = change.after as Row;
        if (
            object?.parent === undefined ||
            change.operation !== "update" ||
            PARENT_COLUMNS.every((column) => before[column] === after[column])
        ) {
            return [];
        }

        return this.#dependentsOf(object, identifier((change.key as Row).id));
    }

    /** List the rows whose access an object's access decides. */
    async #dependentsOf(object: ObjectType, id: string): Promise<RowKey[]> {
        // walk the types level by level
        const keys: RowKey[] = [];
        let level = new Map<ObjectType, string[]>([[object, [id]]]);
        while (level.size > 0) {
            const next = new Map<ObjectType, string[]>();
            for (const [type, ids] of level) {
                // collect rows and subtrees
                const held = [...ids, ...(await this.#subtree(type, ids))];
                keys.push(
                    ...held.map((entry) => ({ table: type.table as Table, key: { id: entry } })),
                );

                // continue to durable children and attachments
                for (const child of this.#server.objects.filter(
                    (object) => object.storage === "durable",
                )) {
                    const isAttached = type.holds(child);
                    if ((type.same(child.parent?.object) && !child.same(type)) || isAttached) {
                        next.set(child, [
                            ...(next.get(child) ?? []),
                            ...(await this.#children(child, type, held)),
                        ]);
                    }
                }
            }
            level = next;
        }

        return keys;
    }

    /** Read the proper descendants of some rows of a tree type. */
    async #subtree(object: ObjectType, ids: readonly string[]): Promise<string[]> {
        // read descendants in batches
        const path = object.tree?.ancestors;
        const below: string[] = [];
        for (let start = 0; path !== undefined && start < ids.length; start += DEPENDENT_IDS) {
            const batch = ids.slice(start, start + DEPENDENT_IDS);
            const rows = await this.#server.database
                .select({ id: path.descendant })
                .from(path)
                .where(
                    and(
                        eq(path.scope, this.#scope),
                        or(...batch.map((entry) => eq(path.ancestor, entry))),
                        gt(path.depth, 0),
                    ),
                );
            below.push(...rows.map((row) => identifier(row.id)));
        }

        return below;
    }

    /** Read the identifiers of the children of some rows. */
    async #children(
        child: ObjectType,
        parent: ObjectType,
        parents: readonly string[],
    ): Promise<string[]> {
        // read children in batches
        const table = child.table as Table & Record<string, never>;
        const columns = table[TABLE].columns;
        const hosted =
            child.parent?.object === "any"
                ? [
                      eq(columns.parentPackageId!, parent.policy.definition.packageId),
                      eq(columns.parentType!, parent.name),
                  ]
                : [];
        const found: string[] = [];
        for (let start = 0; start < parents.length; start += DEPENDENT_IDS) {
            const batch = parents.slice(start, start + DEPENDENT_IDS);
            const rows = (await this.#server.database
                .select({ id: table.id })
                .from(table)
                .where(
                    and(
                        child.inScope(this.#scope),
                        ...hosted,
                        or(...batch.map((entry) => eq(columns.parentId!, entry))),
                    ),
                )) as { id: string }[];
            found.push(...rows.map((row) => identifier(row.id)));
        }

        return found;
    }

    /** Resolve the caller's access again and reset derived state. */
    async #authorize(): Promise<void> {
        // admit again and reset
        this.#authorization = await this.#admit();
        this.#key = undefined;
        this.#until = this.#authorization.access.until;
        if (this.#storage === "ephemeral") {
            this.#decided = await this.#server.database.log.position();
        }
        this.watches = this.#storage === "durable" ? this.#server.authorizer.watch(this.chain) : [];
    }

    /** The caller's access, resolved again once access changes. */
    get authorization(): Authorization {
        return this.#authorization;
    }

    /** The scope and the scopes containing it, nearest first. */
    get chain(): string[] {
        return this.#authorization.chain;
    }

    /** Read the listing permission deciding a table's rows. */
    #listed(
        table: Table,
    ): { readonly object: ObjectType; readonly permission: Permission } | "query" {
        // leave the journal and the copies' records to their queries
        if (table === this.#server.journal.table || table === replica) {
            return "query";
        }

        // read the listing permission
        return this.#objects.listed(table);
    }

    /** Share one grant reader among the scope's subscribers at a position. */
    #reader(position: LogPosition) {
        return this.#server.reader(this.#authorization.access, this.#decided ?? position);
    }
}

/** Read an identifier a row holds. */
function identifier(value: unknown): string {
    return schema.string().parse(value);
}
