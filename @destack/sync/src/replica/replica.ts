import {
    Change,
    and,
    asc,
    count,
    defineTable,
    eq,
    gt,
    gte,
    index,
    integer,
    json,
    inArray,
    Key,
    type Insert,
    lt,
    not,
    or,
    sql,
    text,
    TABLE,
    type DatabaseConnection,
    type SQL,
    type Table,
    Condition,
    Order,
    Scalar,
    DatabaseError,
    Log,
    Snapshot,
    type LogPosition,
    type Row,
    type ColumnValue,
    type QueryOptions,
    type Relation,
    Relations,
} from "@destack/db";
import { SyncError } from "../error/error.ts";
import { schema, canonicalize, found, zip } from "@destack/schema";
import { Page, type ResultChange, type RowChange } from "../query/page.ts";
import { Subscription, type Resumption } from "./shape.ts";
import type { Query, Item, AggregateRow } from "../query/query.ts";
import { Node } from "../query/node.ts";
import type { Prediction } from "../prediction/prediction.ts";
import { EVERYONE } from "../feed/audience.ts";
import { changesThroughLog, Dataflow } from "../dataflow/dataflow.ts";
import { Filter } from "../dataflow/filter.ts";
import { Run } from "../dataflow/run.ts";
import { Trace, type Group, type Upstream } from "../dataflow/upstream.ts";
import { View } from "../dataflow/view.ts";
import { scopeTable } from "../scope/table.ts";

/** The staged pages a run's completion reads at once. */
const STAGED_BATCH = 16;

/** The kept keys a snapshot's completion reads at once. */
const PRUNE_BATCH = 1000;

/** A group's values as a kept group's canonical JSON text has them. */
const GroupValues = schema.record(schema.string(), Scalar);

/** A subscription as a copy records it: what it follows, without where it resumes. */
const SubscriptionRecord = Subscription.omit({
    after: true,
    previous: true,
    refresh: true,
    origin: true,
});
/** A subscription as a copy records it. */
export type SubscriptionRecord = schema.Infer<typeof SubscriptionRecord>;

/** The copies a database keeps, with their source and home positions. */
export const replica = defineTable(
    "replica",
    {
        /** The copy's name. */
        name: text("name").primaryKey(),
        /** The scope whose rows the copy keeps. */
        scope: text("scope").primaryKey(),
        /** The source log's epoch, absent before the first complete snapshot. */
        epoch: text("epoch"),
        /** The source log sequence, absent before the first complete snapshot. */
        sequence: integer("sequence"),
        /** The home log's epoch the rows reflect, absent when the source is their home. */
        originEpoch: text("origin_epoch"),
        /** The home log sequence the rows reflect, absent when the source is their home. */
        originSequence: integer("origin_sequence"),
        /** The subscription the copy completed a run of, absent for a copy no subscription names. */
        subscription: json("subscription", SubscriptionRecord),
        /** The scopes the copy reads, nearest first, absent until its source sends them. */
        scopes: json("scopes", schema.array(schema.string())),
        /** The layout digest of the copied tables' logged columns. */
        layout: text("layout"),
        /** The copied tables' SQL names, recorded when the copy registers. */
        tables: json("tables", schema.array(schema.string())).notNull(),
        /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
        confirmedAt: integer("confirmed_at").notNull(),
    },
    {
        log: {},
    },
);

/** The staged pages of a copy's run in progress. */
export const replicaPage = defineTable("replica_page", {
    /** The copy's name. */
    name: text("name").primaryKey(),
    /** The copy's scope. */
    scope: text("scope").primaryKey(),
    /** The page's place in the run, from zero. */
    index: integer("index").primaryKey(),
    /** The page as the source sent it. */
    page: json("page", Page).notNull(),
});

/** The aggregate groups a copy keeps. */
export const replicaResult = defineTable(
    "replica_result",
    {
        /** The copy's name. */
        name: text("name").primaryKey(),
        /** The copy's scope. */
        scope: text("scope").primaryKey(),
        /** The aggregate query, by its path of names. */
        query: text("query").primaryKey(),
        /** The group's values as canonical JSON text. */
        group: text("group").primaryKey(),
        /** The group's measures by name. */
        values: json("values", schema.record(schema.string(), Scalar)).notNull(),
        /** The rows of the group. */
        rows: integer("rows").notNull(),
        /** Each average's sum and count of present values, by measure name. */
        parts: json(
            "parts",
            schema.record(
                schema.string(),
                schema.object({ sum: Scalar, count: schema.number().int() }),
            ),
        ),
    },
    {
        log: {},
    },
);

/** The rows each copy keeps on the follower, with the columns it hides on each. */
export const replicaRow = defineTable(
    "replica_row",
    {
        /** The copy's name. */
        name: text("name").primaryKey(),
        /** The copy's scope. */
        scope: text("scope").primaryKey(),
        /** The row's table, by SQL name. */
        table: text("table").primaryKey(),
        /** The row's key. */
        key: text("key").primaryKey(),
        /** The columns the copy hides on the row, by property. */
        concealed: json("concealed", schema.array(schema.string())).notNull(),
    },
    {
        constraints: (row) => [index("replica_row_table").on(row.table, row.key)],
    },
);

/** The tables of a database with copies. */
export const replicaTables: readonly Table[] = [replica, replicaPage, replicaResult, replicaRow];

/** A local copy of one scope's rows in a remote database's tables. */
export class Replica {
    /** The copy's name. */
    readonly name: string;
    /** The scope whose rows the copy keeps. */
    readonly scope: string;
    /** The copied tables, parents first. */
    readonly tables: readonly Table[];
    /** The condition on each table's copied rows. */
    readonly where: ReadonlyMap<Table, Condition>;
    /** The scopes of each table whose rows live outside the copy's scope, or in several scopes. */
    readonly scopes: ReadonlyMap<Table, readonly string[]>;
    /** The tables whose rows are copied from whichever scope they live in. */
    readonly everywhere: ReadonlySet<Table>;
    /** The parent tables of each table copied across scopes: each of its rows lives in the scope of a parent row's `id`. */
    readonly within: ReadonlyMap<Table, readonly Table[]>;
    /** Whether the source may keep a copy itself, which the copy follows to its home. */
    readonly isRelayed: boolean;
    /** The source tables whose rows the copy projects into rows of its own, by SQL name. */
    readonly #projectors: ReadonlyMap<string, Projector>;
    /** The copied tables, by SQL name. */
    readonly #copied: ReadonlyMap<string, Table>;
    /** The layout digest of the copied tables' logged columns. */
    readonly #layout: Promise<string>;

    /** Define a copy of a scope's rows in some tables. */
    constructor(definition: {
        readonly name: string;
        readonly scope: string;
        readonly tables: readonly Table[];
        readonly where?: ReadonlyMap<Table, Condition>;
        readonly scopes?: ReadonlyMap<Table, readonly string[]>;
        readonly everywhere?: ReadonlySet<Table>;
        readonly within?: ReadonlyMap<Table, readonly Table[]>;
        readonly isRelayed?: boolean;
        readonly projectors?: readonly Projector[];
    }) {
        // keep the identity
        this.name = definition.name;
        this.scope = definition.scope;
        this.tables = definition.tables;
        this.where = definition.where ?? new Map();
        this.scopes = definition.scopes ?? new Map();
        this.everywhere = definition.everywhere ?? new Set();
        this.within = definition.within ?? new Map();
        this.isRelayed = definition.isRelayed ?? true;
        this.#projectors = new Map(
            (definition.projectors ?? []).map((projector) => [
                projector.source[TABLE].sqlName,
                projector,
            ]),
        );
        this.#copied = new Map(this.tables.map((table) => [table[TABLE].sqlName, table]));
        this.#layout = Log.layout(this.tables);
    }

    /** The queries a fenced source's capture reads: every copied table, unlogged ones included. */
    get captured(): Record<string, Query> {
        // refuse tables copied across scopes
        if (this.within.size > 0) {
            throw new SyncError(
                "INVALID_SCOPE",
                `copy ${this.name} copies tables across scopes, which no capture reads`,
            );
        }

        return Object.fromEntries(
            this.tables.map((table) => [table[TABLE].sqlName, this.#query(table)]),
        );
    }

    /** The queries the copy follows, by table name: its logged tables, and the source's own copy record when relayed. */
    get queries(): Record<string, Query> {
        // follow each logged table outside a parent's rows
        const queries = this.tables
            .filter((table) => !this.within.has(table) && table[TABLE].retention !== "none")
            .map((table): [string, Query] => [
                table[TABLE].sqlName,
                { ...this.#query(table), ...this.#children(table), relations: this.#relations },
            ]);

        // follow the source's own copy record when relayed
        if (this.isRelayed) {
            queries.push([
                replica[TABLE].sqlName,
                { table: replica, scopes: [this.scope], where: { name: this.name } },
            ]);
        }

        return Object.fromEntries(queries);
    }

    /** Build the includes of the tables copied across scopes under a parent table, recursively. */
    #children(parent: Table): { with?: Record<string, QueryOptions> } {
        // include each table living in this table's rows by its relation from the parent
        const children = this.tables.filter(
            (table) => this.within.get(table)?.includes(parent) === true,
        );
        if (children.length === 0) {
            return {};
        }

        return {
            with: Object.fromEntries(
                children.map((table) => [table[TABLE].sqlName, this.#children(table)]),
            ),
        };
    }

    /** The relations from each parent table to the tables whose rows live in its rows' scopes. */
    get #relations(): Relations {
        // relate each copied table to its children by their scope column
        const relations = new Map<Table, Record<string, Relation>>();
        for (const table of this.tables) {
            for (const parent of this.within.get(table) ?? []) {
                const where = this.where.get(table);
                relations.set(parent, {
                    ...relations.get(parent),
                    [table[TABLE].sqlName]: {
                        table,
                        cardinality: "many",
                        on: { kind: "key", column: "scope", parent: "id" },
                        ...(where === undefined ? {} : { where }),
                    },
                });
            }
        }

        return new Relations(relations);
    }

    /** Build the query reading one copied table's rows in its scope. */
    #query(table: Table): Query {
        const where = this.where.get(table);
        const scopes = this.scopes.get(table) ?? [this.scope];

        return {
            table,
            scopes: this.everywhere.has(table) ? "every" : scopes,
            ...(where === undefined ? {} : { where }),
        };
    }

    /** Match the rows of a table keyed by a text `id` that a named copy includes, as SQL. */
    static includes(name: string, table: Table): SQL {
        const sqlName = table[TABLE].sqlName;
        const key = sql`'["' || ${sqlName} || '","' || ${table[TABLE].column("id")} || '"]'`;

        return sql`EXISTS (SELECT 1 FROM ${replicaRow} WHERE ${replicaRow.name} = ${name} AND ${replicaRow.table} = ${sqlName} AND ${replicaRow.key} = ${key})`;
    }

    /** List the keys of some rows of a table that a named copy includes. */
    static async keysIncluded(
        database: DatabaseConnection,
        name: string,
        table: Table,
        rows: readonly Row[],
    ): Promise<ReadonlySet<string>> {
        const included = await database
            .select({ key: replicaRow.key })
            .from(replicaRow)
            .where(
                and(
                    eq(replicaRow.name, name),
                    eq(replicaRow.table, table[TABLE].sqlName),
                    inArray(
                        replicaRow.key,
                        rows.map((row) => Key.name(table, row)),
                    ),
                ),
            );

        return new Set(included.map((row) => row.key));
    }

    /** Report whether a database copies a scope. */
    static async isCopied(database: DatabaseConnection, scope: string): Promise<boolean> {
        const [copy] = await database
            .select({ name: replica.name })
            .from(replica)
            .where(eq(replica.scope, scope))
            .limit(1);

        return copy !== undefined;
    }

    /** Wait until every copy of a scope reflects its home up to a position, returning false once the signal aborts. */
    static async reach(
        database: DatabaseConnection,
        scope: string,
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<boolean> {
        return database.log.until(async () => {
            // read the home position of each copy keeping the scope
            const keeping = await Replica.#keeping(database, [scope]);
            const copies = keeping.map(({ record }) => Replica.#origin(record));

            // refuse a position of an outdated history
            if (copies.some((copy) => copy !== undefined && copy.position.epoch > position.epoch)) {
                throw new DatabaseError(
                    "STALE_EPOCH",
                    `${scope} started a new epoch after the position`,
                );
            }

            return (
                copies.length > 0 &&
                copies.every(
                    (copy) =>
                        copy !== undefined &&
                        copy.position.epoch === position.epoch &&
                        copy.position.sequence >= position.sequence,
                )
            );
        }, signal);
    }

    /** Wait until the copy reflects its home up to a position, returning false once the signal aborts. */
    async reach(
        database: DatabaseConnection,
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<boolean> {
        return database.log.until(async () => {
            // read the copy's home position
            const record = await this.#record(database);
            const origin = record === undefined ? undefined : Replica.#origin(record);

            // refuse a position of an outdated history
            if (origin !== undefined && origin.position.epoch > position.epoch) {
                throw new DatabaseError(
                    "STALE_EPOCH",
                    `${this.scope} started a new epoch after the position`,
                );
            }

            return (
                origin !== undefined &&
                origin.position.epoch === position.epoch &&
                origin.position.sequence >= position.sequence
            );
        }, signal);
    }

    /** Report whether the database keeps the copy's record. */
    async isRecorded(database: DatabaseConnection): Promise<boolean> {
        return (await this.#record(database)) !== undefined;
    }

    /** Refuse relaying a copy that has no position yet. */
    static async requireRelayable(
        database: DatabaseConnection,
        name: string,
        scope: string,
    ): Promise<void> {
        const [record] = await database
            .select()
            .from(replica)
            .where(and(eq(replica.name, name), eq(replica.scope, scope)));
        if (record !== undefined && Replica.#origin(record) === undefined) {
            throw new SyncError("STALE", `copy of ${scope} holds no position yet`);
        }
    }

    /**
     * Refuse publishing the copy to a subscriber whose rows the database copies into one of the copy's tables.
     *
     * A table copies only from its owner, through relays that pass copies onward, so no copied row returns to its source or home.
     */
    async requireAcyclic(database: DatabaseConnection, origin: string): Promise<void> {
        // read the database's copies with the tables each copies
        const records = await database
            .select({
                name: replica.name,
                scope: replica.scope,
                epoch: replica.epoch,
                originEpoch: replica.originEpoch,
                tables: replica.tables,
            })
            .from(replica);
        // refuse a copy from the subscriber keeping one of the copy's tables, with rows or without yet
        const [cyclic] = records.flatMap((record) => {
            const shared = record.tables.find((table) => this.#copied.has(table));
            const isFromOrigin = record.epoch === origin || record.originEpoch === origin;

            return shared !== undefined && isFromOrigin ? [{ record, shared }] : [];
        });
        if (cyclic !== undefined) {
            const table = found(this.#copied, cyclic.shared)[TABLE].name;
            throw new SyncError(
                "CYCLE",
                `copy ${this.name} of ${this.scope} would publish ${table} back to ${origin}, its source in copy ${cyclic.record.name} of ${cyclic.record.scope}`,
            );
        }
    }

    /** Read the home position and confirmation time of the earliest confirmed complete copy of a shape, by scope. */
    static async origins(
        database: DatabaseConnection,
        shape: string,
        scopes: readonly string[],
    ): Promise<Map<string, Origin>> {
        // read the complete copies of the shape keeping the scopes
        const origins = new Map<string, Origin>();
        for (const { scope, record } of await Replica.#keeping(database, scopes)) {
            const origin = Replica.#origin(record);
            if (origin === undefined || record.subscription?.shape !== shape) {
                continue;
            }

            // keep the copy its home confirmed first because a row stays while any copy includes it
            const kept = origins.get(scope);
            if (kept === undefined || origin.confirmedAt < kept.confirmedAt) {
                origins.set(scope, origin);
            }
        }

        return origins;
    }

    /** Read the records of the copies keeping some scopes, once per scope and copy. */
    static async #keeping(
        database: DatabaseConnection,
        scopes: readonly string[],
    ): Promise<{ readonly scope: string; readonly record: typeof replica.$inferSelect }[]> {
        // read the copies recorded at the scopes, and those including the scopes' own rows
        const keys = new Map(scopes.map((scope) => [Key.name(scopeTable, { scope }), scope]));
        const rows = await database
            .select()
            .from(replica)
            .leftJoin(
                replicaRow,
                and(
                    eq(replicaRow.name, replica.name),
                    eq(replicaRow.scope, replica.scope),
                    eq(replicaRow.table, scopeTable[TABLE].sqlName),
                    inArray(replicaRow.key, [...keys.keys()]),
                ),
            )
            .where(or(inArray(replica.scope, scopes), inArray(replicaRow.key, [...keys.keys()])));

        // pair each copy with the scope it is recorded at and the scope whose row it includes
        const pairs = new Map<string, { scope: string; record: typeof replica.$inferSelect }>();
        for (const { replica: record, replica_row: included } of rows) {
            const kept = [
                ...(scopes.includes(record.scope) ? [record.scope] : []),
                ...(included === null ? [] : [found(keys, included.key)]),
            ];
            for (const scope of kept) {
                pairs.set(`${scope} ${record.name} ${record.scope}`, { scope, record });
            }
        }

        return [...pairs.values()];
    }

    /** Read the source position the copy has, absent before its first snapshot or after a layout change. */
    async position(database: DatabaseConnection): Promise<LogPosition | undefined> {
        return this.#positionOf(await this.#record(database), await this.#layout);
    }

    /** Read the copy's record, absent before it registered. */
    async #record(database: DatabaseConnection): Promise<typeof replica.$inferSelect | undefined> {
        const [record] = await database.select().from(replica).where(this.#match(replica));

        return record;
    }

    /** Read a record's source position, absent before its first snapshot or after a layout change. */
    #positionOf(
        record: typeof replica.$inferSelect | undefined,
        layout: string,
    ): LogPosition | undefined {
        return record === undefined ||
            record.epoch === null ||
            record.sequence === null ||
            record.layout !== layout
            ? undefined
            : { epoch: record.epoch, sequence: record.sequence };
    }

    /** Describe the copy's position, layout, queries and staged pages. */
    async inspect(database: DatabaseConnection): Promise<ReplicaInspection> {
        // read the record, staged pages and groups
        const row = await this.#record(database);
        const [staged] = await database
            .select({ pages: count() })
            .from(replicaPage)
            .where(this.#match(replicaPage));
        const [results] = await database
            .select({ groups: count() })
            .from(replicaResult)
            .where(this.#match(replicaResult));
        if (staged === undefined || results === undefined) {
            throw new TypeError("a count read returned no row");
        }

        return {
            name: this.name,
            scope: this.scope,
            ...(row?.epoch === null || row?.sequence === null || row === undefined
                ? {}
                : { position: { epoch: row.epoch, sequence: row.sequence } }),
            ...(row?.originEpoch === null || row?.originSequence === null || row === undefined
                ? {}
                : { origin: { epoch: row.originEpoch, sequence: row.originSequence } }),
            isLaidOut: row?.layout === (await this.#layout),
            ...(row?.subscription === null || row === undefined
                ? {}
                : { subscription: row.subscription }),
            ...(row === undefined ? {} : { confirmedAt: row.confirmedAt }),
            staged: staged.pages,
            results: results.groups,
        };
    }

    /** Read the subscription the copy completed a run of, absent before its first run or for a copy no subscription names. */
    async subscribed(database: DatabaseConnection): Promise<SubscriptionRecord | undefined> {
        const record = await this.#record(database);

        return record?.subscription ?? undefined;
    }

    /** Resume a subscription from the copy: from its position for the same parameters, reshaped from the parameters it reflects, or from a snapshot. */
    async resume(database: DatabaseConnection, subscription?: Subscription): Promise<Resumption> {
        // resume a copy no subscription names from its position
        const after = await this.position(database);
        const recorded = await this.subscribed(database);
        const origin = await database.log.epoch();
        if (after === undefined || subscription === undefined) {
            return after === undefined ? { origin } : { after, origin };
        }

        // resume the same subscription, reshape one of other parameters, and snapshot another
        const { parameters, ...identity } = subscribedOf(subscription);
        if (recorded === undefined) {
            return { origin };
        }
        const { parameters: reflected, ...recordedIdentity } = recorded;
        if (canonicalize(recordedIdentity) !== canonicalize(identity)) {
            return { origin };
        }

        return canonicalize(reflected) === canonicalize(parameters)
            ? { after, origin }
            : { after, previous: reflected, origin };
    }

    /** Read the scope chain the copy reads, nearest first: its own scope alone until its source sends it. */
    async chain(database: DatabaseConnection): Promise<readonly string[]> {
        const record = await this.#record(database);

        return record?.scopes ?? [this.scope];
    }

    /** Read a query's rows from the copy, predictions included. */
    async rows(
        database: DatabaseConnection,
        name: string,
        query: Omit<Query, "scopes">,
        prediction?: Prediction,
    ): Promise<readonly Item[]> {
        // run the query over the copy at its latest position
        const dataflow = new Dataflow(
            { [name]: { ...query, scopes: [this.scope] } },
            {
                audience: EVERYONE,
                database,
                upstream: this.upstream(database, prediction),
                changesThrough: changesThroughLog(database),
                isMaterialized: true,
            },
        );
        await dataflow.load(await View.latest(database));

        return dataflow.read(name);
    }

    /** Read an aggregate query's groups from the copy, predictions included. */
    async results(
        database: DatabaseConnection,
        name: string,
        query: Omit<Query, "scopes">,
        prediction?: Prediction,
    ): Promise<AggregateRow[]> {
        const node = new Node(name, { ...query, scopes: [this.scope] }, [this.scope]);
        const groups = await this.#groups(database, node, prediction);

        return groups.map(({ group, values }) => ({ group, values }));
    }

    /** Measure relations and aggregates through the source's groups, predictions included. */
    upstream(database: DatabaseConnection, prediction: Prediction | undefined): Upstream {
        return {
            watches: [{ table: replicaResult, scopes: [this.scope] }],
            groups: (node) => this.#groups(database, node, prediction),
            groupOf: (change) => {
                // skip changes of other copies' groups
                const row = Change.image(change);
                if (
                    change.table !== replicaResult ||
                    row["name"] !== this.name ||
                    row["scope"] !== this.scope
                ) {
                    return undefined;
                }

                // read the group's query and values
                const query = row["query"];
                const group = row["group"];
                if (typeof query !== "string" || typeof group !== "string") {
                    throw new TypeError("a kept group lacks its query or values");
                }

                return { query, group: GroupValues.parse(JSON.parse(group)) };
            },
        };
    }

    /** Read a node's source groups with the uncounted predictions. */
    async #groups(
        database: DatabaseConnection,
        node: Node,
        prediction: Prediction | undefined,
    ): Promise<Group[]> {
        // read the source groups
        const rows = await database
            .select({
                group: replicaResult.group,
                values: replicaResult.values,
                rows: replicaResult.rows,
                parts: replicaResult.parts,
            })
            .from(replicaResult)
            .where(and(this.#match(replicaResult), eq(replicaResult.query, node.name)));
        const groups = new Map<string, GroupPrediction>(
            rows.map((row) => [
                row.group,
                {
                    group: GroupValues.parse(JSON.parse(row.group)),
                    values: { ...row.values },
                    rows: row.rows,
                    parts: { ...row.parts },
                },
            ]),
        );

        // decide the predicted images of the node's table
        const table = node.table[TABLE].sqlName;
        const predicted = (
            prediction === undefined ? [] : await prediction.predicted(database)
        ).filter((change) => change.table === table);
        const images = predicted.flatMap((change) => predictedImages(node, change));
        if (images.length > 0) {
            const run = new Run(await View.latest(database), EVERYONE, "collect");
            const filter = new Filter(node, new Trace(this.upstream(database, prediction)));
            await filter.prepare(
                images.map(({ image }) => image),
                run,
            );

            // add the candidates to their groups
            for (const { image, sign } of images) {
                if (filter.isCandidate(image, run)) {
                    predict(node, groups, filter.resolve(image, run), sign);
                }
            }
        }

        return [...groups.values()].filter((group) => group.rows > 0);
    }

    /** Keep the copied rows as the database's own. */
    async promote(database: DatabaseConnection): Promise<void> {
        await database.transaction(async (transaction) => {
            // require a complete copy
            if ((await this.position(transaction)) === undefined) {
                throw new SyncError("STALE", `copy of ${this.scope} holds no position yet`);
            }

            // drop the copy's record, staged pages, groups and included rows
            await transaction.delete(replicaPage).where(this.#match(replicaPage));
            await transaction.delete(replicaResult).where(this.#match(replicaResult));
            await transaction.delete(replicaRow).where(this.#match(replicaRow));
            await transaction.delete(replica).where(this.#match(replica));
        });
    }

    /** Record the copy before its first snapshot, with the subscription it follows when given. */
    async register(database: DatabaseConnection, subscription?: Subscription): Promise<void> {
        await database
            .insert(replica)
            .values({
                name: this.name,
                scope: this.scope,
                epoch: null,
                sequence: null,
                subscription: subscription === undefined ? null : subscribedOf(subscription),
                tables: [...this.#copied.keys()],
                confirmedAt: Date.now(),
            })
            .onConflictDoNothing();
    }

    /** Drop the copy: delete the rows it wrote that no other copy includes, retract its projections and forget its record. */
    async drop(database: DatabaseConnection): Promise<void> {
        await database.transaction(
            async (transaction) => {
                await transaction.log.asReplica(async () => {
                    // take every included row out of the copy with children first
                    for (const table of this.tables.toReversed()) {
                        let keys = await this.#keys(transaction, table);
                        while (keys.length > 0) {
                            await this.#exclude(transaction, table, keys);
                            keys = await this.#keys(transaction, table);
                        }
                    }

                    // retract every projected row of the copied scope
                    for (const projector of this.#projectors.values()) {
                        await projector.prune(transaction, this.scope, new Set());
                    }

                    // forget the groups, staged pages and record
                    await transaction.delete(replicaResult).where(this.#match(replicaResult));
                    await transaction.delete(replicaPage).where(this.#match(replicaPage));
                    await transaction.delete(replica).where(this.#match(replica));
                });
            },
            { constraints: "deferred" },
        );
    }

    /** List the subscriptions of the copies a database keeps, those no subscription names left out. */
    static async subscriptions(database: DatabaseConnection): Promise<SubscriptionRecord[]> {
        const records = await database.select({ subscription: replica.subscription }).from(replica);

        return records.flatMap((record) =>
            record.subscription === null ? [] : [record.subscription],
        );
    }

    /** Apply one stream of pages, staging each run until it completes. */
    async *apply(
        database: DatabaseConnection,
        pages: AsyncIterable<Page> | Iterable<Page>,
        options: ApplyOptions = {},
    ): AsyncGenerator<Page> {
        // drop an earlier stream's staged pages
        await database.delete(replicaPage).where(this.#match(replicaPage));
        let staged = 0;
        let isSnapshot = false;
        for await (const page of pages) {
            // drop staged pages a snapshot replaces
            if (page.reset && staged > 0) {
                await database.delete(replicaPage).where(this.#match(replicaPage));
                staged = 0;
            }
            if (staged === 0) {
                isSnapshot = page.reset;
            }

            // apply the run a page completes
            if (page.complete) {
                await this.#complete(database, page, { staged, isSnapshot }, options);
                staged = 0;
            }
            // stage a page within a run
            else {
                await database
                    .insert(replicaPage)
                    .values({ name: this.name, scope: this.scope, index: staged, page });
                staged += 1;
            }
            yield page;
        }
    }

    /** Apply a source's pages until the signal aborts, resuming each stream that ends where the copy is. */
    async follow(
        database: DatabaseConnection,
        source: (from: Resumption, signal: AbortSignal) => AsyncIterable<Page>,
        signal: AbortSignal,
        options: ApplyOptions = {},
    ): Promise<void> {
        // apply each stream from where the copy is until aborted
        await this.register(database, options.subscription);
        while (!signal.aborted) {
            let isReceived = false;
            const pages = source(await this.resume(database, options.subscription), signal);
            const applied = this.apply(database, pages, options);
            while ((await applied.next()).done !== true) {
                isReceived = true;
            }

            // reject a stream that ends without a page
            if (!isReceived && !signal.aborted) {
                throw new SyncError(
                    "INVALID_STREAM",
                    `the source of ${this.scope} ended a stream without a page`,
                );
            }
        }
    }

    /** Apply a run's pages in one transaction, rebasing local predictions. */
    async #complete(
        database: DatabaseConnection,
        page: Page,
        run: Stage,
        options: ApplyOptions,
    ): Promise<void> {
        // apply and rebase in one transaction
        await database.transaction(
            (transaction) => this.#applyRun(transaction, page, run, options),
            { constraints: "deferred" },
        );
    }

    /** Apply a run's pages inside a transaction around reverted local predictions. */
    async #applyRun(
        transaction: DatabaseConnection,
        page: Page,
        run: Stage,
        options: ApplyOptions,
    ): Promise<void> {
        // rebase onto pages that touch a prediction
        const rebased = await rebasedBy(transaction, page, run, options.prediction);

        // require the recorded epoch outside a snapshot
        const record = await this.#record(transaction);
        const recorded = this.#positionOf(record, await this.#layout);
        if (!run.isSnapshot && recorded !== undefined && recorded.epoch !== page.position.epoch) {
            throw new DatabaseError(
                "STALE_EPOCH",
                `page of epoch ${page.position.epoch} continues a copy of ${recorded.epoch}`,
            );
        }

        // write the source's rows after reverting predictions, under the source's origin
        await transaction.log.asReplica(async () => {
            await rebased?.revert(transaction);
            await this.#writeRun(transaction, page, run, record, options);
        }, page.position.epoch);

        // predict again what the source lacks
        await rebased?.replay(transaction, {
            position: page.position,
            isSnapshot: run.isSnapshot,
        });
    }

    /** Write a run's pages and record the new position. */
    async #writeRun(
        transaction: DatabaseConnection,
        page: Page,
        run: Stage,
        record: typeof replica.$inferSelect | undefined,
        options: ApplyOptions,
    ): Promise<void> {
        // let go of every group on a snapshot
        if (run.isSnapshot) {
            await transaction.delete(replicaResult).where(this.#match(replicaResult));
        }

        // write the staged and completing pages and prune what a snapshot left out
        const delivered = run.isSnapshot ? this.#deliveries() : undefined;
        const staging = await this.#writeStaged(transaction, run.staged, delivered, options);
        const relayed = (await this.#write(transaction, page, delivered, options)) ?? staging;
        if (delivered !== undefined) {
            await this.#pruneUndelivered(transaction, delivered);
        }

        // record the new position
        await this.#advance(transaction, page, run, record, relayed, options);
    }

    /** Record a completed run's position, home position, scopes and subscription, and drop its staged pages. */
    async #advance(
        transaction: DatabaseConnection,
        page: Page,
        run: Stage,
        record: typeof replica.$inferSelect | undefined,
        relayed: Origin | undefined,
        { subscription }: ApplyOptions,
    ): Promise<void> {
        // take the home position the rows reflect
        const isHome = relayed === undefined && (run.isSnapshot || record?.originEpoch === null);
        const advanced = {
            ...page.position,
            ...originColumns(relayed, isHome),
            ...(page.scopes === undefined ? {} : { scopes: page.scopes }),
            ...(subscription === undefined ? {} : { subscription: subscribedOf(subscription) }),
            layout: await this.#layout,
            tables: [...this.#copied.keys()],
        };

        // write the record and drop the staged pages
        await transaction
            .insert(replica)
            .values({
                name: this.name,
                scope: this.scope,
                confirmedAt: record?.confirmedAt ?? Date.now(),
                ...advanced,
            })
            .onConflictDoUpdate({ target: [replica.name, replica.scope], set: advanced });
        await transaction.delete(replicaPage).where(this.#match(replicaPage));
    }

    /** Start collecting the keys a snapshot delivers, by copied and projected table. */
    #deliveries(): Map<string, Set<string>> {
        const projected = [...this.#projectors.values()].map((projector) => projector.source);

        return new Map(
            [...this.tables, ...projected].map((table) => [
                table[TABLE].sqlName,
                new Set<string>(),
            ]),
        );
    }

    /** Write the staged pages in batches, returning the last home position a relaying source sent. */
    async #writeStaged(
        transaction: DatabaseConnection,
        staged: number,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
        options: ApplyOptions,
    ): Promise<Origin | undefined> {
        let relayed: Origin | undefined;
        for (let start = 0; start < staged; start += STAGED_BATCH) {
            // read one batch of staged pages in order
            const batch = await transaction
                .select({ page: replicaPage.page })
                .from(replicaPage)
                .where(
                    and(
                        this.#match(replicaPage),
                        gte(replicaPage.index, start),
                        lt(replicaPage.index, start + STAGED_BATCH),
                    ),
                )
                .orderBy(asc(replicaPage.index));

            // write each page
            for (const row of batch) {
                relayed = (await this.#write(transaction, row.page, delivered, options)) ?? relayed;
            }
        }

        return relayed;
    }

    /** Remove the rows a snapshot left out, children first, and retract their projections. */
    async #pruneUndelivered(
        transaction: DatabaseConnection,
        delivered: ReadonlyMap<string, Set<string>>,
    ): Promise<void> {
        for (const table of this.tables.toReversed()) {
            await this.#prune(transaction, table, found(delivered, table[TABLE].sqlName));
        }
        for (const [name, projector] of this.#projectors) {
            await projector.prune(transaction, this.scope, found(delivered, name));
        }
    }

    /** Write a page's changes in commit order, returning the home position a relaying source sent. */
    async #write(
        transaction: DatabaseConnection,
        page: Page,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
        { open }: ApplyOptions,
    ): Promise<Origin | undefined> {
        // batch each table's writes and each projected table's rows
        const { batches, projected, origin } = await this.#collect(page, delivered, open);

        // take the removed rows out of the copy children first and write the kept rows parents first
        const ordered = this.tables.flatMap((table): [Table, Batch][] => {
            const batch = batches.get(table);

            return batch === undefined ? [] : [[table, batch]];
        });
        for (const [table, batch] of ordered.toReversed()) {
            const keys = batch.removed.map((row) => Key.name(table, row));
            await this.#exclude(transaction, table, keys);
        }
        for (const [table, batch] of ordered) {
            await this.#writeKept(transaction, table, batch);
        }

        // project the collected rows and keep the groups
        for (const [projector, rows] of projected) {
            await projector.write(transaction, this.scope, rows.kept, rows.removed);
        }
        await this.#keep(transaction, page.results ?? []);

        return origin;
    }

    /** Collect a page's changes into table batches and projected rows. */
    async #collect(
        page: Page,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
        open: ApplyOptions["open"],
    ): Promise<{
        readonly batches: Map<Table, Batch>;
        readonly projected: Map<Projector, ProjectionPage>;
        readonly origin: Origin | undefined;
    }> {
        // collect each change in commit order
        const batches = new Map<Table, Batch>();
        const projected = new Map<Projector, ProjectionPage>();
        let origin: Origin | undefined;
        for (const change of page.changes) {
            const table = this.#copied.get(change.table);
            const projector = this.#projectors.get(change.table);

            // take a relaying source's copy record as the home position
            if (change.table === replica[TABLE].sqlName) {
                origin = this.#relayed(change);
                continue;
            }

            // collect a projected row's change and note its key for a snapshot's retractions
            if (projector !== undefined) {
                collectProjected(projected, projector, change, table, delivered);
            }

            // skip a projected table the copy keeps no rows of and refuse one outside the copy
            if (table === undefined) {
                if (projector !== undefined) {
                    continue;
                }
                throw new SyncError(
                    "INVALID_STREAM",
                    `page names a table outside copy ${this.name}: ${change.table}`,
                );
            }
            // stage a copied row's change with its host-bound values opened
            else {
                await collectBatched(batches, table, change, delivered, open);
            }
        }

        return { batches, projected, origin };
    }

    /** Read the home position a relaying source's copy record sends. */
    #relayed(change: RowChange): Origin {
        const origin =
            change.operation === "delete"
                ? undefined
                : Replica.#origin(
                      replica[TABLE].selectSchema().parse(replica[TABLE].decode(change.row)),
                  );
        if (origin === undefined) {
            throw new DatabaseError("STALE_EPOCH", `source stopped copying ${this.scope}`);
        }

        return origin;
    }

    /** Write a table's kept rows of one page and record them as included. */
    async #writeKept(transaction: DatabaseConnection, table: Table, batch: Batch): Promise<void> {
        // write the kept rows with hidden columns cleared and record them as included
        await transaction.upsert(
            table,
            await this.#conceal(transaction, table, batch.kept, batch.hidden),
        );
        await this.#include(transaction, table, batch.kept, batch.hidden);
    }

    /** Keep or let go of groups, or of every group of a query, a batch of groups per statement. */
    async #keep(transaction: DatabaseConnection, results: readonly ResultChange[]): Promise<void> {
        // collect each group's last change and let go of a whole query at once
        const groups = new Map<string, GroupWrite>();
        for (const result of results) {
            const key = {
                name: this.name,
                scope: this.scope,
                query: result.query,
                group: canonicalize(result.group),
            };
            // let go of every group of a query
            if (result.values === null && result.group === null) {
                await writeGroups(transaction, groups);
                await transaction
                    .delete(replicaResult)
                    .where(and(this.#match(replicaResult), eq(replicaResult.query, result.query)));
            }
            // let go of an empty group
            else if (result.values === null) {
                groups.set(JSON.stringify(key), { key, row: null });
            }
            // refuse a kept group without a row count
            else if (result.rows === undefined) {
                throw new SyncError(
                    "INVALID_STREAM",
                    `page has a group of ${result.query} without its rows`,
                );
            }
            // keep the group's values
            else {
                const row = {
                    ...key,
                    values: { ...result.values },
                    rows: result.rows,
                    parts: result.parts === undefined ? null : { ...result.parts },
                };
                groups.set(JSON.stringify(key), { key, row });
            }
        }
        await writeGroups(transaction, groups);
    }

    /** Take the rows a completed snapshot left out of the copy, a batch at a time in key order. */
    async #prune(database: DatabaseConnection, table: Table, delivered: ReadonlySet<string>) {
        // read a table copied across scopes by the rows the copy includes
        if (this.within.has(table) || this.everywhere.has(table)) {
            await this.#pruneIncluded(database, table, delivered);
        }
        // read any other table by the rows of the copy's scope and condition
        else {
            await this.#pruneSelected(database, table, delivered);
        }
    }

    /** Take the rows a snapshot left out of a table copied across scopes. */
    async #pruneIncluded(
        database: DatabaseConnection,
        table: Table,
        delivered: ReadonlySet<string>,
    ): Promise<void> {
        const name = table[TABLE].sqlName;
        let last: string | undefined;
        do {
            // read the next batch of included keys
            const rows = await database
                .select({ key: replicaRow.key })
                .from(replicaRow)
                .where(
                    and(
                        this.#match(replicaRow),
                        eq(replicaRow.table, name),
                        last === undefined ? undefined : gt(replicaRow.key, last),
                    ),
                )
                .orderBy(asc(replicaRow.key))
                .limit(PRUNE_BATCH);
            last = rows.at(-1)?.key;

            // take out the rows the snapshot left out
            const stale = rows.map((row) => row.key).filter((key) => !delivered.has(key));
            await this.#exclude(database, table, stale);
        } while (last !== undefined);
    }

    /** Take the rows a snapshot left out of a table within the copy's scope and condition. */
    async #pruneSelected(
        database: DatabaseConnection,
        table: Table,
        delivered: ReadonlySet<string>,
    ): Promise<void> {
        // select the keys of the copy's rows in key order
        const order = Order.complete([], table);
        const fields = Object.fromEntries(
            table[TABLE].key.map((name) => [name, table[TABLE].column(name)]),
        );
        const where = Condition.render(
            {
                AND: [
                    Node.scoped(this.scopes.get(table) ?? [this.scope]),
                    this.where.get(table) ?? {},
                ],
            },
            table,
        );
        let last: Row | undefined;
        do {
            // read the next batch of keys
            const rows = await database
                .select(fields)
                .from(table)
                .where(and(where, last === undefined ? undefined : Order.after(order, table, last)))
                .orderBy(...Order.render(order, table))
                .limit(PRUNE_BATCH);
            last = rows.at(-1);

            // take out the rows the snapshot left out
            const stale = rows
                .map((row) => Key.name(table, row))
                .filter((name) => !delivered.has(name));
            await this.#exclude(database, table, stale);
        } while (last !== undefined);
    }

    /** Clear the columns the copy hides on some rows, keeping a column another copy shows. */
    async #conceal(
        database: DatabaseConnection,
        table: Table,
        rows: readonly Row[],
        hidden: readonly (readonly string[])[],
    ): Promise<Row[]> {
        // read what the other copies hide on the rows hiding columns
        const hiding = zip(rows, hidden)
            .filter(([, columns]) => columns.length > 0)
            .map(([row]) => row);
        const others = await this.#others(
            database,
            table,
            hiding.map((row) => Key.name(table, row)),
        );

        // read the stored values of the rows another copy includes
        const shared = hiding.filter((row) => others.has(Key.name(table, row)));
        const stored = new Map(
            (await storedRows(database, table, shared)).map((row) => [Key.name(table, row), row]),
        );

        return zip(rows, hidden).map(([row, columns]) => {
            // keep a hidden column as stored while another copy shows it
            const name = Key.name(table, row);
            const elsewhere = others.get(name) ?? [];
            const values = columns.map((column): [string, ColumnValue] => [
                column,
                elsewhere.some((other) => !other.includes(column))
                    ? (stored.get(name)?.[column] ?? null)
                    : null,
            ]);

            return { ...row, ...Object.fromEntries(values) };
        });
    }

    /** Read the columns the other copies hide on some rows, by row key. */
    async #others(
        database: DatabaseConnection,
        table: Table,
        keys: readonly string[],
    ): Promise<Map<string, string[][]>> {
        // read the other copies' records of the rows, a batch at a time
        const name = table[TABLE].sqlName;
        const others = new Map<string, string[][]>();
        for (let start = 0; start < keys.length; start += PRUNE_BATCH) {
            const records = await database
                .select({ key: replicaRow.key, concealed: replicaRow.concealed })
                .from(replicaRow)
                .where(
                    and(
                        not(this.#match(replicaRow)),
                        eq(replicaRow.table, name),
                        inArray(replicaRow.key, keys.slice(start, start + PRUNE_BATCH)),
                    ),
                );
            for (const row of records) {
                others.set(row.key, [...(others.get(row.key) ?? []), row.concealed]);
            }
        }

        return others;
    }

    /** Record the rows the copy includes and the columns it hides on each. */
    async #include(
        database: DatabaseConnection,
        table: Table,
        rows: readonly Row[],
        hidden: readonly (readonly string[])[],
    ): Promise<void> {
        const name = table[TABLE].sqlName;
        await database.upsert(
            replicaRow,
            zip(rows, hidden).map(([row, concealed]) => ({
                name: this.name,
                scope: this.scope,
                table: name,
                key: Key.name(table, row),
                concealed: [...concealed],
            })),
        );
    }

    /** Take some rows out of the copy, deleting those no copy includes any longer. */
    async #exclude(
        database: DatabaseConnection,
        table: Table,
        keys: readonly string[],
    ): Promise<void> {
        // take the rows out of the copy
        const name = table[TABLE].sqlName;
        for (let start = 0; start < keys.length; start += PRUNE_BATCH) {
            const batch = keys.slice(start, start + PRUNE_BATCH);
            const selected = and(
                this.#match(replicaRow),
                eq(replicaRow.table, name),
                inArray(replicaRow.key, batch),
            );
            const shown = await database
                .select({ key: replicaRow.key, concealed: replicaRow.concealed })
                .from(replicaRow)
                .where(selected);
            await database.delete(replicaRow).where(selected);

            // delete the rows no other copy includes
            const others = await this.#others(database, table, batch);
            const deleted = batch
                .filter((key) => !others.has(key))
                .map((key) => Key.parse(table, key));
            await database.remove(table, deleted);

            // clear the columns only this copy showed on the rows the others keep
            await clearShown(database, table, shown, others);
        }
    }

    /** Read a record's home position, absent before its first snapshot. */
    static #origin(record: typeof replica.$inferSelect): Origin | undefined {
        // skip a copy without a position
        if (record.epoch === null || record.sequence === null) {
            return undefined;
        }

        // take a relayed copy's origin, or the source position
        const position =
            record.originEpoch === null || record.originSequence === null
                ? { epoch: record.epoch, sequence: record.sequence }
                : { epoch: record.originEpoch, sequence: record.originSequence };

        return { position, confirmedAt: record.confirmedAt };
    }

    /** Match the copy's rows of a bookkeeping table keyed by name and scope. */
    #match(
        table: typeof replica | typeof replicaPage | typeof replicaResult | typeof replicaRow,
    ): SQL {
        return and(eq(table.name, this.name), eq(table.scope, this.scope));
    }

    /** Read a batch of the keys the copy includes in a table. */
    async #keys(database: DatabaseConnection, table: Table): Promise<string[]> {
        const rows = await database
            .select({ key: replicaRow.key })
            .from(replicaRow)
            .where(and(this.#match(replicaRow), eq(replicaRow.table, table[TABLE].sqlName)))
            .limit(PRUNE_BATCH);

        return rows.map((row) => row.key);
    }
}

/** A run of staged pages a completing page applies. */
interface Stage {
    /** The staged pages before the completing one. */
    readonly staged: number;
    /** Whether the run starts with a snapshot. */
    readonly isSnapshot: boolean;
}

/** A group as predictions change it. */
type GroupPrediction = Group & { rows: number };

/** A table's staged writes of one page. */
interface Batch {
    /** The rows the copy keeps. */
    readonly kept: Row[];
    /** The columns the copy hides on each kept row. */
    readonly hidden: string[][];
    /** The rows the copy lets go. */
    readonly removed: Row[];
}

/** A projected table's rows of one page. */
interface ProjectionPage {
    /** The rows the projection keeps. */
    readonly kept: Row[];
    /** The rows the projection lets go. */
    readonly removed: Row[];
}

/** A group's last change in one page. */
interface GroupWrite {
    /** The group's key. */
    readonly key: Key<typeof replicaResult>;
    /** The group's kept row or null for a let go group. */
    readonly row: Insert<typeof replicaResult> | null;
}

/** Collect a projected row's change. */
function collectProjected(
    projected: Map<Projector, ProjectionPage>,
    projector: Projector,
    change: RowChange,
    table: Table | undefined,
    delivered: ReadonlyMap<string, Set<string>> | undefined,
): void {
    // note the key for a snapshot's retractions
    const row = projector.source[TABLE].decode(change.row);
    if (delivered !== undefined && table === undefined) {
        found(delivered, change.table).add(Key.name(projector.source, row));
    }

    // add the row to the kept or removed ones
    const rows = projected.get(projector) ?? { kept: [], removed: [] };
    projected.set(projector, rows);
    (change.operation === "delete" ? rows.removed : rows.kept).push(row);
}

/** Collect a copied row's change into its table's batch. */
async function collectBatched(
    batches: Map<Table, Batch>,
    table: Table,
    change: RowChange,
    delivered: ReadonlyMap<string, Set<string>> | undefined,
    open: ApplyOptions["open"],
): Promise<void> {
    // open the row and note its key for a snapshot's retractions
    const decoded = table[TABLE].decode(change.row);
    const row =
        open === undefined || change.operation === "delete" ? decoded : await open(table, decoded);
    if (delivered !== undefined) {
        found(delivered, change.table).add(Key.name(table, row));
    }

    // add the row to the kept or removed ones
    const batch = batches.get(table) ?? { kept: [], hidden: [], removed: [] };
    batches.set(table, batch);
    if (change.operation === "delete") {
        batch.removed.push(row);
    } else {
        batch.kept.push(row);
        batch.hidden.push(change.concealed ?? []);
    }
}

/** Write the collected group changes and clear them. */
async function writeGroups(
    transaction: DatabaseConnection,
    groups: Map<string, GroupWrite>,
): Promise<void> {
    // take the collected changes
    const entries = [...groups.values()];
    groups.clear();

    // remove the let go groups and write the kept ones
    await transaction.remove(
        replicaResult,
        entries.flatMap(({ key, row }) => (row === null ? [key] : [])),
    );
    await transaction.upsert(
        replicaResult,
        entries.flatMap(({ row }) => (row === null ? [] : [row])),
    );
}

/** Record a subscription without where it resumes. */
function subscribedOf(subscription: Subscription): SubscriptionRecord {
    const {
        after: _after,
        previous: _previous,
        refresh: _refresh,
        origin: _origin,
        ...subscribed
    } = subscription;

    return subscribed;
}

/** Add a predicted row image to its group, or take it away. */
function predict(node: Node, groups: Map<string, GroupPrediction>, row: Row, sign: 1 | -1): void {
    // find or start the row's group
    const group = node.groupOf(row);
    const key = canonicalize(group);
    const known = groups.get(key) ?? { group, values: node.emptyValues(), rows: 0, parts: {} };
    groups.set(key, known);
    known.rows += sign;

    // add to counts, sums and averages, and pass extremes on additions
    const aggregate = node.aggregate;
    if (aggregate === undefined) {
        throw new TypeError(`node ${node.name} predicts groups without an aggregate`);
    }
    for (const [name, measure] of Object.entries(aggregate.values)) {
        const current = known.values[name] ?? null;
        // count every row
        if (measure.function === "count") {
            known.values[name] = Number(current ?? 0) + sign;
            continue;
        }

        // read the measured column's present value
        const value = row[measure.column] ?? null;
        const present = value === null ? null : node.scalar(measure.column, value);
        if (present === null) {
            continue;
        }
        // sum present values
        if (measure.function === "sum") {
            known.values[name] = add(current, present, sign);
        }
        // average present values through their sum and count
        else if (measure.function === "avg") {
            const part = known.parts[name] ?? { sum: 0, count: 0 };
            const next = { sum: add(part.sum, present, sign), count: part.count + sign };
            known.parts[name] = next;
            known.values[name] = next.count === 0 ? null : Number(next.sum) / next.count;
        }
        // pass an added value that beats the extreme
        else if (sign === 1) {
            const order =
                current === null
                    ? undefined
                    : Order.values(
                          node.fromJson(measure.column, present),
                          node.fromJson(measure.column, current),
                      );
            if (order === undefined || (measure.function === "min" ? order < 0 : order > 0)) {
                known.values[name] = present;
            }
        }
    }
}

/** Take the predictions a completing page rebases: those of a snapshot, a staged run, groups or a touched table. */
async function rebasedBy(
    transaction: DatabaseConnection,
    page: Page,
    run: Stage,
    prediction: Prediction | undefined,
): Promise<Prediction | undefined> {
    // skip a copy without predictions
    if (prediction === undefined) {
        return undefined;
    }

    // rebase onto a snapshot, a staged run, groups, a reached mutation or a predicted table
    const tables = new Set(page.changes.map((change) => change.table));
    const isRebased =
        run.isSnapshot ||
        run.staged > 0 ||
        (page.results ?? []).length > 0 ||
        (await prediction.isReached(transaction, page.position)) ||
        (tables.size > 0 && (await prediction.touches(transaction, tables)));

    return isRebased ? prediction : undefined;
}

/** Read the stored rows of a table by the keys of some rows. */
function storedRows(
    database: DatabaseConnection,
    table: Table,
    keys: readonly Row[],
): Promise<Row[]> {
    const primary = table[TABLE].key;

    return Snapshot.live(database).select(
        table,
        primary,
        keys.map((key) => primary.map((column) => key[column])),
    );
}

/** Clear the columns a copy showed alone on the rows other copies keep, given the columns each copy hides. */
async function clearShown(
    database: DatabaseConnection,
    table: Table,
    shown: readonly { readonly key: string; readonly concealed: readonly string[] }[],
    others: ReadonlyMap<string, readonly (readonly string[])[]>,
): Promise<void> {
    // take the columns every other copy hides and this copy showed
    const cleared = shown.flatMap(({ key, concealed }) => {
        const hidden = others.get(key) ?? [];
        const columns = (hidden[0] ?? []).filter(
            (column) =>
                !concealed.includes(column) && hidden.every((each) => each.includes(column)),
        );

        return columns.length === 0 ? [] : [{ key: Key.parse(table, key), columns }];
    });

    // write the stored rows back with those columns cleared
    const clearing = new Map(cleared.map(({ key, columns }) => [Key.name(table, key), columns]));
    const stored = await storedRows(
        database,
        table,
        cleared.map(({ key }) => key),
    );
    await database.upsert(
        table,
        stored.map((row) => {
            const columns = found(clearing, Key.name(table, row));

            return { ...row, ...Object.fromEntries(columns.map((column) => [column, null])) };
        }),
    );
}

/** Add a JSON value to a sum, or take it away. */
function add(sum: Scalar, value: Scalar, sign: 1 | -1): Scalar {
    return typeof sum === "string" || typeof value === "string"
        ? String(BigInt(sum ?? 0) + BigInt(sign) * BigInt(value ?? 0))
        : Number(sum ?? 0) + sign * Number(value);
}

/** How a copy applies its source's pages. */
export interface ApplyOptions {
    /** The client's predictions to rebase. */
    readonly prediction?: Prediction;
    /** The subscription the copy follows, recorded as it completes a run. */
    readonly subscription?: Subscription;
    /** Open the host-bound values a fenced source sealed to this copy. */
    readonly open?: (table: Table, row: Row) => Promise<Row>;
}

/** How a copy projects a source table's rows into rows of its own, as a read model of them. */
export interface Projector {
    /** The source table whose rows the copy projects. */
    readonly source: Table;
    /** Write the target rows of a source scope's kept rows, and retract those of removed ones, which hold only their key. */
    write(
        transaction: DatabaseConnection,
        scope: string,
        kept: readonly Row[],
        removed: readonly Row[],
    ): Promise<void>;
    /** Retract the target rows of a source scope's rows a snapshot left out, given the keys it delivered. */
    prune(
        transaction: DatabaseConnection,
        scope: string,
        delivered: ReadonlySet<string>,
    ): Promise<void>;
}

/** The home position of a copy's rows. */
export interface Origin {
    /** The home log position the rows reflect. */
    readonly position: LogPosition;
    /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
    readonly confirmedAt: number;
}

/** The inspection of a copy. */
export interface ReplicaInspection {
    /** The copy's name. */
    readonly name: string;
    /** The scope it copies. */
    readonly scope: string;
    /** The recorded source position. */
    readonly position?: LogPosition;
    /** The home position, absent when the source is the home. */
    readonly origin?: LogPosition;
    /** Whether its tables have the copied layout. */
    readonly isLaidOut: boolean;
    /** The subscription it completed a run of, absent for a copy no subscription names. */
    readonly subscription?: SubscriptionRecord;
    /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
    readonly confirmedAt?: number;
    /** The staged pages. */
    readonly staged: number;
    /** The aggregate groups it keeps. */
    readonly results: number;
}

/** Read a predicted change's images of a node's rows: its row before, signed negative, and after, signed positive. */
function predictedImages(
    node: Node,
    change: RowChange,
): { readonly image: Row; readonly sign: -1 | 1 }[] {
    // require an update's before
    const table = node.table[TABLE];
    const row = table.decode(change.row);
    if (change.operation === "update" && change.before === undefined) {
        throw new SyncError(
            "INVALID_STREAM",
            `predicted update of ${table.sqlName} lacks its before`,
        );
    }

    // take the images the operation has
    let before: Row | undefined = row;
    if (change.operation === "insert") {
        before = undefined;
    } else if (change.operation === "update" && change.before !== undefined) {
        before = table.decode(change.before);
    }
    const after = change.operation === "delete" ? undefined : row;

    return [
        ...(before === undefined ? [] : [{ image: before, sign: -1 as const }]),
        ...(after === undefined ? [] : [{ image: after, sign: 1 as const }]),
    ];
}

/** Write the home position a copy's rows reflect: a relaying source's, its own at home, or none to keep. */
function originColumns(
    relayed: Origin | undefined,
    isHome: boolean,
):
    | {
          readonly originEpoch: string | null;
          readonly originSequence: number | null;
          readonly confirmedAt: number;
      }
    | {} {
    // take a relaying source's position
    if (relayed !== undefined) {
        return {
            originEpoch: relayed.position.epoch,
            originSequence: relayed.position.sequence,
            confirmedAt: relayed.confirmedAt,
        };
    }
    // mark rows written at home
    else if (isHome) {
        return { originEpoch: null, originSequence: null, confirmedAt: Date.now() };
    }
    // keep the recorded position
    else {
        return {};
    }
}
