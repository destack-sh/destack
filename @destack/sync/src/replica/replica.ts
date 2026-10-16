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
    identifier,
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
    Condition,
    Order,
    Scalar,
    DatabaseError,
    Log,
    Snapshot,
    type LogPosition,
    type Row,
    type ColumnValue,
    type Relation,
    Relations,
    type Transaction,
    type Select,
    Table,
} from "@destack/db";
import type { QuerySelection } from "../query/query.ts";
import { SyncError } from "../error/error.ts";
import { schema, canonicalize, found, zip } from "@destack/schema";
import { Page, type ResultChange, type RowChange } from "../query/page.ts";
import { PublicationAudience, Subscription, type Resumption } from "./publication.ts";
import { Subject } from "../call/subject.ts";
import type { Query, Item } from "../query/query.ts";
import type { AggregateRow } from "../query/aggregate.ts";
import { Node } from "../query/node.ts";
import type { Prediction } from "../prediction/prediction.ts";
import { EVERYONE } from "../feed/audience.ts";
import { Dataflow } from "../dataflow/dataflow.ts";
import { Filter } from "../dataflow/filter.ts";
import { Run } from "../dataflow/run.ts";
import { Trace, type Group, type Upstream } from "../dataflow/upstream.ts";
import { View } from "../dataflow/view.ts";
import { scopeTable } from "../scope/table.ts";
import { Clock } from "@destack/environment";

/** The staged pages a run's completion reads at once. */
const STAGED_BATCH = 16;

/** The kept keys a snapshot's completion reads at once. */
const PRUNE_BATCH = 1000;

/** A group's values as a kept group's canonical JSON text has them. */
const GroupValues = schema.record(schema.string(), Scalar);

/** A subscription as a replica records it: what it follows, without where it resumes. */
const SubscriptionRecord = Subscription.omit({
    after: true,
    previous: true,
    refresh: true,
    origin: true,
});
/** A subscription as a replica records it. */
export type SubscriptionRecord = schema.Infer<typeof SubscriptionRecord>;

/** The replicas a database keeps: what each replicates and from which subscription. */
export const replica = defineTable(
    "replica",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The scope whose rows the replica keeps. */
        scope: text("scope").primaryKey(),
        /** The publication of the last completed run's subscription, absent without one. */
        publication: text("publication"),
        /** The scope the subscription's follower serves. */
        below: text("below"),
        /** The package of the principal the source decides the rows for. */
        subjectPackageId: identifier("subject_package_id", "package"),
        /** The type of the principal decided for. */
        subjectType: text("subject_type"),
        /** The scope of the principal decided for. */
        subjectScope: text("subject_scope"),
        /** The identifier of the principal decided for. */
        subjectId: text("subject_id"),
        /** The relation whose members the subject set is, absent for one principal. */
        subjectRelation: text("subject_relation"),
        /** The subscription's parameters, which its publication parses. */
        parameters: json("parameters", schema.jsonObject()),
        /** Whose access its source decides the rows by, absent without a subscription. */
        audience: text("audience").validate(PublicationAudience),
        /** The scopes the replica reads, nearest first, absent until its source sends them. */
        scopes: json("scopes", schema.array(schema.string())),
        /** The layout digest of the replicated tables' logged columns. */
        layout: text("layout"),
        /** The replicated tables' SQL names, recorded when the replica registers. */
        tables: json("tables", schema.array(schema.string())).notNull(),
    },
    {
        log: {},
    },
);

/** How far each replica of a database has come: source and home positions and last confirmation. */
export const replicaPosition = defineTable(
    "replica_position",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The scope whose rows the replica keeps. */
        scope: text("scope").primaryKey(),
        /** The source log's epoch, absent before the first complete snapshot. */
        epoch: text("epoch"),
        /** The source log sequence, absent before the first complete snapshot. */
        sequence: integer("sequence"),
        /** The home log's epoch the rows reflect, absent when the source is their home. */
        originEpoch: text("origin_epoch"),
        /** The home log sequence the rows reflect, absent when the source is their home. */
        originSequence: integer("origin_sequence"),
        /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
        confirmedAt: integer("confirmed_at").notNull(),
    },
    {
        log: {},
    },
);

/** A replica's registration with its position. */
type ReplicaRecord = Select<typeof replica> &
    Omit<Select<typeof replicaPosition>, "name" | "scope">;

/** Join a replica's registration to its position, by the replica's name and scope. */
const POSITIONED = and(
    eq(replicaPosition.name, replica.name),
    eq(replicaPosition.scope, replica.scope),
);

/** The staged pages of a replica's run in progress. */
export const replicaPage = defineTable("replica_page", {
    /** The replica's name. */
    name: text("name").primaryKey(),
    /** The replica's scope. */
    scope: text("scope").primaryKey(),
    /** The page's place in the run, from zero. */
    index: integer("index").primaryKey(),
    /** The page as the source sent it. */
    page: json("page", Page).notNull(),
});

/** The aggregate groups a replica keeps. */
export const replicaResult = defineTable(
    "replica_result",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The replica's scope. */
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
        averages: json(
            "averages",
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

/** The rows each replica keeps on the follower, with the columns it hides on each. */
export const replicaRow = defineTable(
    "replica_row",
    {
        /** The replica's name. */
        name: text("name").primaryKey(),
        /** The replica's scope. */
        scope: text("scope").primaryKey(),
        /** The row's table, by SQL name. */
        table: text("table").primaryKey(),
        /** The row's key. */
        key: text("key").primaryKey(),
        /** The columns the replica hides on the row, by property. */
        concealed: json("concealed", schema.array(schema.string())).notNull(),
    },
    {
        constraints: (row) => [index("replica_row_table").on(row.table, row.key)],
    },
);

/** The tables of a database with replicas. */
export const replicaTables: readonly Table[] = [
    replica,
    replicaPosition,
    replicaPage,
    replicaResult,
    replicaRow,
];

/** A local replica of one scope's rows in a remote database's tables. */
export class Replica {
    /** The replica's name. */
    readonly name: string;
    /** The scope whose rows the replica keeps. */
    readonly scope: string;
    /** The replicated tables, parents first. */
    readonly tables: readonly Table[];
    /** The condition on each table's replicated rows. */
    readonly where: ReadonlyMap<Table, Condition>;
    /** The scopes of each table whose rows live outside the replica's scope, or in several. */
    readonly scopes: ReadonlyMap<Table, readonly string[]>;
    /** The tables whose rows are replicated from their own scope. */
    readonly everywhere: ReadonlySet<Table>;
    /** The parent tables of each cross-scope table: each row lives in a parent row's scope. */
    readonly within: ReadonlyMap<Table, readonly Table[]>;
    /** Whether the source may keep a replica itself, which the replica follows to its home. */
    readonly isRelayed: boolean;
    /** The replicated tables, by SQL name. */
    readonly #replicated: ReadonlyMap<string, Table>;
    /** The layout digest of the replicated tables' logged columns. */
    readonly #layout: Promise<string>;

    /** Define a replica of a scope's rows in some tables. */
    constructor(definition: {
        /** The replica's name. */
        readonly name: string;
        /** The scope whose rows the replica keeps. */
        readonly scope: string;
        /** The replicated tables, parents first. */
        readonly tables: readonly Table[];
        /** The condition on each table's replicated rows, none by default. */
        readonly where?: ReadonlyMap<Table, Condition>;
        /** The scopes of each table with rows outside the replica's scope, none by default. */
        readonly scopes?: ReadonlyMap<Table, readonly string[]>;
        /** The tables whose rows are replicated from their own scope, none by default. */
        readonly everywhere?: ReadonlySet<Table>;
        /** The parent tables of each table replicated across scopes, none by default. */
        readonly within?: ReadonlyMap<Table, readonly Table[]>;
        /** Whether the source may keep a replica itself, true by default. */
        readonly isRelayed?: boolean;
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
        this.#replicated = new Map(this.tables.map((table) => [table[TABLE].sqlName, table]));
        this.#layout = Log.layout(this.tables);
    }

    /** The queries a fenced source's capture reads: every replicated table, unlogged ones too. */
    get captured(): Record<string, Query> {
        // refuse tables replicated across scopes
        if (this.within.size > 0) {
            throw new SyncError(
                "INVALID_SCOPE",
                `replica ${this.name} replicates tables across scopes, but no capture can read them`,
            );
        }

        return Object.fromEntries(
            this.tables.map((table) => [table[TABLE].sqlName, this.#query(table)]),
        );
    }

    /** The queries the replica follows by table: its logged tables and the source's record. */
    get queries(): Record<string, Query> {
        // follow each logged table outside a parent's rows
        const queries = this.tables
            .filter((table) => !this.within.has(table) && table[TABLE].retention !== "none")
            .map((table): [string, Query] => [
                table[TABLE].sqlName,
                { ...this.#query(table), ...this.#children(table), relations: this.#relations },
            ]);

        // follow the source's own replica position when relayed
        if (this.isRelayed) {
            queries.push([
                replicaPosition[TABLE].sqlName,
                { table: replicaPosition, scopes: [this.scope], where: { name: this.name } },
            ]);
        }

        return Object.fromEntries(queries);
    }

    /** Build the includes of tables replicated across scopes under a parent table, recursively. */
    #children(parent: Table): { with?: Record<string, QuerySelection> } {
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
        // relate each replicated table to its children by their scope column
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

    /** Build the query reading one replicated table's rows in its scope. */
    #query(table: Table): Query {
        const where = this.where.get(table);
        const scopes = this.scopes.get(table) ?? [this.scope];

        return {
            table,
            scopes: this.everywhere.has(table) ? "every" : scopes,
            ...(where === undefined ? {} : { where }),
        };
    }

    /** Match the rows of a table keyed by a text `id` in the replicas a reader sees, as SQL. */
    static includes(table: Table, below: string, caller: Subject | undefined): SQL {
        // match the row's key in the replicas the reader sees
        const sqlName = table[TABLE].sqlName;
        const key = sql`'["' || ${sqlName} || '","' || ${table[TABLE].column("id")} || '"]'`;
        const seen = Replica.#seen(below, caller);

        return sql`EXISTS (SELECT 1 FROM ${replicaRow} INNER JOIN ${replica} ON ${replica.name} = ${replicaRow.name} AND ${replica.scope} = ${replicaRow.scope} WHERE ${replicaRow.table} = ${sqlName} AND ${replicaRow.key} = ${key} AND ${seen})`;
    }

    /** List the keys of some rows of a table that the replicas a reader sees include. */
    static async keysIncluded(
        database: DatabaseConnection,
        table: Table,
        rows: readonly Row[],
        below: string,
        caller: Subject | undefined,
    ): Promise<ReadonlySet<string>> {
        const included = await database
            .select({ key: replicaRow.key })
            .from(replicaRow)
            .innerJoin(
                replica,
                and(eq(replica.name, replicaRow.name), eq(replica.scope, replicaRow.scope)),
            )
            .where(
                and(
                    eq(replicaRow.table, table[TABLE].sqlName),
                    inArray(
                        replicaRow.key,
                        rows.map((row) => Key.name(table, row)),
                    ),
                    Replica.#seen(below, caller),
                ),
            );

        return new Set(included.map((row) => row.key));
    }

    /** Match the replicas a reader below a scope sees: decided for its readers, or the caller. */
    static #seen(below: string, caller: Subject | undefined): SQL {
        // match the replicas served below the scope, decided for its readers or for the calling principal
        const isBelow = eq(replica.below, below);
        const isReader = eq(replica.audience, "reader");
        const isCaller =
            caller === undefined
                ? undefined
                : and(
                      eq(replica.audience, "caller"),
                      eq(replica.subjectPackageId, caller.packageId),
                      eq(replica.subjectType, caller.type),
                      eq(replica.subjectScope, caller.scope),
                      eq(replica.subjectId, caller.id),
                  );

        return and(isBelow, or(isReader, isCaller));
    }

    /** Take rows a database pruned out of every replica including them. */
    static async release(
        database: DatabaseConnection,
        table: Table,
        rows: readonly Row[],
    ): Promise<void> {
        // keep no bookkeeping in a database without replicas
        if (!database.tables.includes(replicaRow) || rows.length === 0) {
            return;
        }

        // delete the rows' entries of every replica
        await database.delete(replicaRow).where(
            and(
                eq(replicaRow.table, table[TABLE].sqlName),
                inArray(
                    replicaRow.key,
                    rows.map((row) => Key.name(table, row)),
                ),
            ),
        );
    }

    /** List the tables of the database's replicas by scope, or of those a reader sees. */
    static async tables(
        database: DatabaseConnection,
        ...reader: [] | [below: string, caller: Subject | undefined]
    ): Promise<readonly { readonly scope: string; readonly tables: readonly string[] }[]> {
        // list no replicas in a database without them
        if (!database.tables.includes(replica)) {
            return [];
        }

        return database
            .select({ scope: replica.scope, tables: replica.tables })
            .from(replica)
            .where(reader.length === 0 ? undefined : Replica.#seen(...reader));
    }

    /** List the scopes whose rows of a table the replicas have, or those a reader sees. */
    static async scopes(
        database: DatabaseConnection,
        table: Table,
        ...reader: [] | [below: string, caller: Subject | undefined]
    ): Promise<string[]> {
        // take the scopes of the replicas with the table
        const sqlName = table[TABLE].sqlName;
        const records = await Replica.tables(database, ...reader);

        return [
            ...new Set(
                records.filter((kept) => kept.tables.includes(sqlName)).map(({ scope }) => scope),
            ),
        ];
    }

    /** Report whether a database replicates a scope, or the scope's rows of one table. */
    static async isReplicated(
        snapshot: Pick<Snapshot, "read">,
        scope: string,
        table?: Table,
    ): Promise<boolean> {
        // read the replicas of the scope with the tables they keep
        const records = await snapshot.read(replica).findMany({ where: { scope } });
        const name = table?.[TABLE].sqlName;

        return records.some((record) => name === undefined || record.tables.includes(name));
    }

    /** Wait until every replica of a scope reflects its home up to a position, or until aborted. */
    static async wait(
        database: DatabaseConnection,
        scope: string,
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<boolean> {
        return database.log.until(async () => {
            // read the home position of each replica keeping the scope
            const keeping = await Replica.#keeping(database, [scope]);
            const origins = keeping.map(({ record }) => Replica.#origin(record));

            // refuse a position of an outdated history
            if (
                origins.some(
                    (origin) => origin !== undefined && origin.position.epoch > position.epoch,
                )
            ) {
                throw new DatabaseError(
                    "STALE_EPOCH",
                    `${scope} started a new epoch after the position`,
                );
            }

            return (
                origins.length > 0 &&
                origins.every(
                    (origin) =>
                        origin !== undefined &&
                        origin.position.epoch === position.epoch &&
                        origin.position.sequence >= position.sequence,
                )
            );
        }, signal);
    }

    /** Wait until the replica reflects its home up to a position, or the signal aborts. */
    async wait(
        database: DatabaseConnection,
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<boolean> {
        return database.log.until(async () => {
            // read the replica's home position
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

    /** Report whether the database keeps the replica's record. */
    async isRecorded(snapshot: Snapshot): Promise<boolean> {
        return (
            (await snapshot
                .read(replica)
                .findFirst({ where: { name: this.name, scope: this.scope } })) !== undefined
        );
    }

    /** Report whether a database's replica of a scope is a complete snapshot, as informers sync. */
    static async isSynced(
        database: DatabaseConnection,
        name: string,
        scope: string,
    ): Promise<boolean> {
        const record = await Replica.#find(database, name, scope);

        return record !== undefined && Replica.#origin(record) !== undefined;
    }

    /** Report whether a replica completed a run of a subscription as it is requested now. */
    static async isCurrent(
        database: DatabaseConnection,
        subscription: Subscription,
    ): Promise<boolean> {
        const record = await Replica.#find(database, subscription.name, subscription.scope);
        const recorded = record === undefined ? undefined : Replica.subscriptionOf(record);

        return (
            record !== undefined &&
            Replica.#origin(record) !== undefined &&
            recorded !== undefined &&
            canonicalize(recorded) === canonicalize(Replica.#subscribed(subscription))
        );
    }

    /** Read a database's record of a named replica of a scope, absent before it follows. */
    static async #find(
        database: DatabaseConnection,
        name: string,
        scope: string,
    ): Promise<ReplicaRecord | undefined> {
        const [record] = await database
            .select({ registration: replica, position: replicaPosition })
            .from(replica)
            .innerJoin(replicaPosition, POSITIONED)
            .where(and(eq(replica.name, name), eq(replica.scope, scope)));

        return record === undefined
            ? undefined
            : Replica.#merge(record.registration, record.position);
    }

    /** Refuse relaying a replica that has no position yet. */
    static async requireRelayable(
        database: DatabaseConnection,
        name: string,
        scope: string,
    ): Promise<void> {
        const record = await Replica.#find(database, name, scope);
        if (record !== undefined && Replica.#origin(record) === undefined) {
            throw new SyncError("STALE", `replica of ${scope} has no position yet`);
        }
    }

    /** Refuse publishing the replica to a subscriber whose rows the database replicates into it. */
    async requireAcyclic(database: DatabaseConnection, origin: string): Promise<void> {
        // read the database's replicas with the tables each replicates
        const records = await database
            .select({
                name: replica.name,
                scope: replica.scope,
                epoch: replicaPosition.epoch,
                originEpoch: replicaPosition.originEpoch,
                tables: replica.tables,
            })
            .from(replica)
            .innerJoin(replicaPosition, POSITIONED);
        // refuse a replica from the subscriber keeping one of the replica's tables, with rows or without yet
        const [cyclic] = records.flatMap((record) => {
            const shared = record.tables.find((table) => this.#replicated.has(table));
            const isFromOrigin = record.epoch === origin || record.originEpoch === origin;

            return shared !== undefined && isFromOrigin ? [{ record, shared }] : [];
        });
        if (cyclic !== undefined) {
            const table = found(this.#replicated, cyclic.shared)[TABLE].name;
            throw new SyncError(
                "CYCLE",
                `replica ${this.name} of ${this.scope} would publish ${table} back to ${origin}, its source in replica ${cyclic.record.name} of ${cyclic.record.scope}`,
            );
        }
    }

    /** Read the earliest confirmed complete replica's home position and confirmation, by scope. */
    static async origins(
        database: DatabaseConnection,
        publication: string,
        scopes: readonly string[],
    ): Promise<Map<string, Origin>> {
        // read the complete replicas of the publication keeping the scopes
        const origins = new Map<string, Origin>();
        for (const { scope, record } of await Replica.#keeping(database, scopes)) {
            const origin = Replica.#origin(record);
            if (origin === undefined || record.publication !== publication) {
                continue;
            }

            // keep the replica its home confirmed first because a row stays while any replica includes it
            const kept = origins.get(scope);
            if (kept === undefined || origin.confirmedAt < kept.confirmedAt) {
                origins.set(scope, origin);
            }
        }

        return origins;
    }

    /** Read the records of the replicas keeping some scopes, once per scope and replica. */
    static async #keeping(
        database: DatabaseConnection,
        scopes: readonly string[],
    ): Promise<{ readonly scope: string; readonly record: ReplicaRecord }[]> {
        // read the replicas recorded at the scopes, and those including the scopes' own rows
        const keys = new Map(scopes.map((scope) => [Key.name(scopeTable, { scope }), scope]));
        const rows = await database
            .select()
            .from(replica)
            .innerJoin(replicaPosition, POSITIONED)
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

        // pair each replica with the scope it is recorded at and the scope whose row it includes
        const pairs = new Map<string, { scope: string; record: ReplicaRecord }>();
        for (const {
            replica: registration,
            replica_position: position,
            replica_row: included,
        } of rows) {
            const record = Replica.#merge(registration, position);
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

    /** Read the replica's source position, absent before a snapshot or after a layout change. */
    async position(database: DatabaseConnection): Promise<LogPosition | undefined> {
        return this.#recordedPosition(await this.#record(database), await this.#layout);
    }

    /** Read the replica's record, absent before it registered. */
    async #record(database: DatabaseConnection): Promise<ReplicaRecord | undefined> {
        return Replica.#find(database, this.name, this.scope);
    }

    /** Read a record's source position, absent before a first snapshot or after a layout change. */
    #recordedPosition(record: ReplicaRecord | undefined, layout: string): LogPosition | undefined {
        return record === undefined ||
            record.epoch === null ||
            record.sequence === null ||
            record.layout !== layout
            ? undefined
            : { epoch: record.epoch, sequence: record.sequence };
    }

    /** Describe the replica's position, layout, queries and staged pages. */
    async inspect(database: DatabaseConnection): Promise<ReplicaInspection> {
        // read the record, its subscription, staged pages and groups
        const row = await this.#record(database);
        const subscription = row === undefined ? undefined : Replica.subscriptionOf(row);
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
            ...(subscription === undefined ? {} : { subscription }),
            ...(row === undefined ? {} : { confirmedAt: row.confirmedAt }),
            staged: staged.pages,
            results: results.groups,
        };
    }

    /** Read the subscription of the replica's last completed run, if any. */
    async subscribed(database: DatabaseConnection): Promise<SubscriptionRecord | undefined> {
        const record = await this.#record(database);

        return record === undefined ? undefined : Replica.subscriptionOf(record);
    }

    /** Resume a subscription from the replica. */
    async resume(database: DatabaseConnection, subscription?: Subscription): Promise<Resumption> {
        // resume a replica without a subscription from its position
        const after = await this.position(database);
        const recorded = await this.subscribed(database);
        const origin = await database.log.epoch();
        if (after === undefined || subscription === undefined) {
            return after === undefined ? { origin } : { after, origin };
        }

        // resume the same subscription, reshape one of other parameters, and snapshot another
        const { parameters, ...identity } = Replica.#subscribed(subscription);
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

    /** Read the replica's scope chain, nearest first: only its scope until its source sends it. */
    async chain(database: DatabaseConnection): Promise<readonly string[]> {
        const record = await this.#record(database);

        return record?.scopes ?? [this.scope];
    }

    /** Read a query's rows from the replica, predictions included. */
    async rows(
        database: DatabaseConnection,
        name: string,
        query: Omit<Query, "scopes">,
        prediction?: Prediction,
    ): Promise<readonly Item[]> {
        // run the query over the replica at its latest position
        const dataflow = new Dataflow(
            { [name]: { ...query, scopes: [this.scope] } },
            {
                audience: EVERYONE,
                database,
                upstream: this.upstream(database, prediction),
                changesThrough: Dataflow.changesThrough(database),
                isMaterialized: true,
            },
        );
        await dataflow.load(await View.latest(database));

        return dataflow.read(name);
    }

    /** Read an aggregate query's groups from the replica, predictions included. */
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
            group: (change) => {
                // skip changes of other replicas' groups
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
                averages: replicaResult.averages,
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
                    averages: { ...row.averages },
                },
            ]),
        );

        // decide the predicted images of the node's table
        const table = node.table[TABLE].sqlName;
        const predicted = (
            prediction === undefined ? [] : await prediction.predicted(database)
        ).filter((change) => change.table === table);
        const images = predicted.flatMap((change) => Replica.#predictedImages(node, change));
        if (images.length > 0) {
            const run = new Run(await View.latest(database), EVERYONE, "collect");
            const filter = new Filter(node, new Trace(this.upstream(database, prediction)));
            const candidates = new Set(
                await filter.prepare(
                    images.map(({ image }) => image),
                    run,
                ),
            );

            // add the candidates to their groups
            for (const { image, sign } of images) {
                if (candidates.has(image)) {
                    Replica.#predictGroups(node, groups, filter.resolve(image, run), sign);
                }
            }
        }

        return [...groups.values()].filter((group) => group.rows > 0);
    }

    /** Keep the replicated rows as the database's own. */
    async promote(transaction: Transaction): Promise<void> {
        // require a complete replica
        const key = { name: this.name, scope: this.scope };
        const registration = await transaction.read(replica).findFirst({ where: key });
        const position = await transaction.read(replicaPosition).findFirst({ where: key });
        const layout = await this.#layout;
        if (
            registration === undefined ||
            position === undefined ||
            this.#recordedPosition(Replica.#merge(registration, position), layout) === undefined
        ) {
            throw new SyncError("STALE", `replica of ${this.scope} has no position yet`);
        }

        // drop the replica's registration and position, and its staged pages, groups and included rows as the transaction commits
        await transaction.write({ table: replica, before: registration, after: null });
        await transaction.write({ table: replicaPosition, before: position, after: null });
        transaction.record(async (database) => {
            await database.delete(replicaPage).where(this.#match(replicaPage));
            await database.delete(replicaResult).where(this.#match(replicaResult));
            await database.delete(replicaRow).where(this.#match(replicaRow));
        });
    }

    /** Record the replica before its first snapshot, with the subscription it follows, if any. */
    async register(database: DatabaseConnection, options: RegisterOptions): Promise<void> {
        const { subscription, audience, clock } = options;
        await database.transaction(async (transaction) => {
            const key = { name: this.name, scope: this.scope };
            await transaction
                .insert(replica)
                .values({
                    ...key,
                    ...Replica.subscriptionColumns(subscription),
                    audience: audience ?? null,
                    tables: [...this.#replicated.keys()],
                })
                .onConflictDoNothing();
            await transaction
                .insert(replicaPosition)
                .values({ ...key, epoch: null, sequence: null, confirmedAt: clock.now() })
                .onConflictDoNothing();
        });
    }

    /** Drop the replica: delete its unshared rows and forget its record. */
    async drop(database: DatabaseConnection): Promise<void> {
        await database.transaction(
            async (transaction) => {
                await transaction.log.asReplica(async () => {
                    // take every included row out of the replica, those of tables it stopped replicating first
                    const record = await this.#record(transaction);
                    await this.#excludeAll(transaction, this.#retired(transaction, record));
                    await this.#excludeAll(transaction, this.tables);

                    // forget the groups, staged pages and record
                    await transaction.delete(replicaResult).where(this.#match(replicaResult));
                    await transaction.delete(replicaPage).where(this.#match(replicaPage));
                    await transaction.delete(replicaPosition).where(this.#match(replicaPosition));
                    await transaction.delete(replica).where(this.#match(replica));
                });
            },
            { constraints: "deferred" },
        );
    }

    /** List the subscriptions of a database's replicas, leaving out replicas without one. */
    static async subscriptions(database: DatabaseConnection): Promise<SubscriptionRecord[]> {
        const records = await database.select().from(replica);

        return records.flatMap((record) => {
            const subscription = Replica.subscriptionOf(record);

            return subscription === undefined ? [] : [subscription];
        });
    }

    /** Apply one stream of pages, staging each run until it completes. */
    async *apply(
        database: DatabaseConnection,
        pages: AsyncIterable<Page> | Iterable<Page>,
        options: ApplyOptions,
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

    /** Apply a source's pages until aborted, resuming each stream ending where the replica is. */
    async follow(
        database: DatabaseConnection,
        source: (from: Resumption, signal: AbortSignal) => AsyncIterable<Page>,
        signal: AbortSignal,
        options: ApplyOptions,
    ): Promise<void> {
        // apply each stream from where the replica is until aborted
        await this.register(database, options);
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
        const rebased = await Replica.#rebased(transaction, page, run, options.prediction);

        // require the recorded epoch outside a snapshot
        const record = await this.#record(transaction);
        const recorded = this.#recordedPosition(record, await this.#layout);
        if (!run.isSnapshot && recorded !== undefined && recorded.epoch !== page.position.epoch) {
            throw new DatabaseError(
                "STALE_EPOCH",
                `page of epoch ${page.position.epoch} continues a replica of ${recorded.epoch}`,
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
        record: ReplicaRecord | undefined,
        options: ApplyOptions,
    ): Promise<void> {
        // let go of every group on a snapshot, and of the rows of tables the replica stopped replicating
        if (run.isSnapshot) {
            await transaction.delete(replicaResult).where(this.#match(replicaResult));
        }
        await this.#excludeAll(transaction, this.#retired(transaction, record));

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

    /** Record a completed run's positions, scopes and subscription, and drop its staged pages. */
    async #advance(
        transaction: DatabaseConnection,
        page: Page,
        run: Stage,
        record: ReplicaRecord | undefined,
        relayed: Origin | undefined,
        { subscription, audience, clock }: ApplyOptions,
    ): Promise<void> {
        // take the home position the rows reflect, and what the replica replicates
        const key = { name: this.name, scope: this.scope };
        const isHome = relayed === undefined && (run.isSnapshot || record?.originEpoch === null);
        const position = {
            ...page.position,
            ...Replica.#originColumns(relayed, isHome, clock.now()),
        };
        const registration = {
            ...(page.scopes === undefined ? {} : { scopes: page.scopes }),
            ...(subscription === undefined ? {} : Replica.subscriptionColumns(subscription)),
            ...(audience === undefined ? {} : { audience }),
            layout: await this.#layout,
            tables: [...this.#replicated.keys()],
        };

        // write the registration, then the position, and drop the staged pages
        await transaction
            .insert(replica)
            .values({ ...key, ...registration })
            .onConflictDoUpdate({ target: [replica.name, replica.scope], set: registration });
        await transaction
            .insert(replicaPosition)
            .values({ ...key, confirmedAt: record?.confirmedAt ?? clock.now(), ...position })
            .onConflictDoUpdate({
                target: [replicaPosition.name, replicaPosition.scope],
                set: position,
            });
        await transaction.delete(replicaPage).where(this.#match(replicaPage));
    }

    /** Resolve the recorded tables the replica no longer replicates, as declared. */
    #retired(database: DatabaseConnection, record: ReplicaRecord | undefined): Table[] {
        // find the recorded tables the replica dropped
        const names = (record?.tables ?? []).filter((name) => !this.#replicated.has(name));
        const tables = database.tables.filter((table) => names.includes(table[TABLE].sqlName));
        const undeclared = names.filter((name) =>
            tables.every((table) => table[TABLE].sqlName !== name),
        );
        if (undeclared.length > 0) {
            throw new TypeError(
                `replica ${this.name} of ${this.scope} has rows in ${undeclared.join(", ")}, but the database no longer declares them`,
            );
        }

        return tables;
    }

    /** Take every row the replica includes out of some tables, children first. */
    async #excludeAll(database: DatabaseConnection, tables: readonly Table[]): Promise<void> {
        for (const table of tables.toReversed()) {
            let keys = await this.#keys(database, table);
            while (keys.length > 0) {
                await this.#exclude(database, table, keys);
                keys = await this.#keys(database, table);
            }
        }
    }

    /** Start collecting the keys a snapshot delivers, by replicated table. */
    #deliveries(): Map<string, Set<string>> {
        return new Map(this.tables.map((table) => [table[TABLE].sqlName, new Set<string>()]));
    }

    /** Write the staged pages in batches, returning a relaying source's last home position. */
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

    /** Remove the rows a snapshot left out, children first. */
    async #pruneUndelivered(
        transaction: DatabaseConnection,
        delivered: ReadonlyMap<string, Set<string>>,
    ): Promise<void> {
        for (const table of this.tables.toReversed()) {
            await this.#prune(transaction, table, found(delivered, table[TABLE].sqlName));
        }
    }

    /** Write a page's changes in commit order, returning a relaying source's home position. */
    async #write(
        transaction: DatabaseConnection,
        page: Page,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
        { open }: ApplyOptions,
    ): Promise<Origin | undefined> {
        // batch each table's writes
        const { batches, origin } = await this.#collect(page, delivered, open);

        // take the removed rows out of the replica children first and write the kept rows parents first
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

        // keep the groups
        await this.#keep(transaction, page.results ?? []);

        return origin;
    }

    /** Collect a page's changes into table batches. */
    async #collect(
        page: Page,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
        open: ApplyOptions["open"],
    ): Promise<{
        readonly batches: Map<Table, Batch>;
        readonly origin: Origin | undefined;
    }> {
        // collect each change in commit order
        const batches = new Map<Table, Batch>();
        let origin: Origin | undefined;
        for (const change of page.changes) {
            const table = this.#replicated.get(change.table);

            // take a relaying source's replica position as the home position
            if (change.table === replicaPosition[TABLE].sqlName) {
                origin = this.#relayed(change);
                continue;
            }

            // refuse a table outside the replica
            if (table === undefined) {
                throw new SyncError(
                    "INVALID_STREAM",
                    `page has a table outside replica ${this.name}: ${change.table}`,
                );
            }
            // stage a replicated row's change with its machine-bound values opened
            else {
                await Replica.#collectBatched(batches, table, change, delivered, open);
            }
        }

        return { batches, origin };
    }

    /** Read the home position a relaying source's replica record sends. */
    #relayed(change: RowChange): Origin {
        const origin =
            change.operation === "delete"
                ? undefined
                : Replica.#origin(
                      Table.selectSchema(replicaPosition).parse(
                          Table.decode(replicaPosition, change.row),
                      ),
                  );
        if (origin === undefined) {
            throw new DatabaseError("STALE_EPOCH", `source stopped replicating ${this.scope}`);
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
                await Replica.#writeGroups(transaction, groups);
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
                    averages: result.averages === undefined ? null : { ...result.averages },
                };
                groups.set(JSON.stringify(key), { key, row });
            }
        }
        await Replica.#writeGroups(transaction, groups);
    }

    /** Take the rows a completed snapshot left out of the replica, a batch at a time by key. */
    async #prune(database: DatabaseConnection, table: Table, delivered: ReadonlySet<string>) {
        // read a table replicated across scopes by the rows the replica includes
        if (this.within.has(table) || this.everywhere.has(table)) {
            await this.#pruneIncluded(database, table, delivered);
        }
        // read any other table by the rows of the replica's scope and condition
        else {
            await this.#pruneSelected(database, table, delivered);
        }
    }

    /** Take the rows a snapshot left out of a table replicated across scopes. */
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

    /** Take the rows a snapshot left out of a table within the replica's scope and condition. */
    async #pruneSelected(
        database: DatabaseConnection,
        table: Table,
        delivered: ReadonlySet<string>,
    ): Promise<void> {
        // select the keys of the replica's rows in key order
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

    /** Clear the columns the replica hides on some rows, keeping a column another replica shows. */
    async #conceal(
        database: DatabaseConnection,
        table: Table,
        rows: readonly Row[],
        hidden: readonly (readonly string[])[],
    ): Promise<Row[]> {
        // read what the other replicas hide on the rows hiding columns
        const hiding = zip(rows, hidden)
            .filter(([, columns]) => columns.length > 0)
            .map(([row]) => row);
        const others = await this.#others(
            database,
            table,
            hiding.map((row) => Key.name(table, row)),
        );

        // read the stored values of the rows another replica includes
        const shared = hiding.filter((row) => others.has(Key.name(table, row)));
        const stored = new Map(
            (await Replica.#storedRows(database, table, shared)).map((row) => [
                Key.name(table, row),
                row,
            ]),
        );

        return zip(rows, hidden).map(([row, columns]) => {
            // keep a hidden column as stored while another replica shows it
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

    /** Read the columns the other replicas hide on some rows, by row key. */
    async #others(
        database: DatabaseConnection,
        table: Table,
        keys: readonly string[],
    ): Promise<Map<string, string[][]>> {
        // read the other replicas' records of the rows, a batch at a time
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

    /** Record the rows the replica includes and the columns it hides on each. */
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

    /** Take some rows out of the replica, deleting those no replica includes any longer. */
    async #exclude(
        database: DatabaseConnection,
        table: Table,
        keys: readonly string[],
    ): Promise<void> {
        // leave the predefined rows to the database, which keeps them identical everywhere
        const owned = keys.filter((key) => !Key.isPredefined(table, Key.parse(table, key)));

        // take the rows out of the replica
        const name = table[TABLE].sqlName;
        for (let start = 0; start < owned.length; start += PRUNE_BATCH) {
            const batch = owned.slice(start, start + PRUNE_BATCH);
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

            // delete the rows no other replica includes
            const others = await this.#others(database, table, batch);
            const deleted = batch
                .filter((key) => !others.has(key))
                .map((key) => Key.parse(table, key));
            await database.remove(table, deleted);

            // clear the columns only this replica showed on the rows the others keep
            await Replica.#clearShown(database, table, shown, others);
        }
    }

    /** Read a record's home position, absent before its first snapshot. */
    static #origin(record: Select<typeof replicaPosition>): Origin | undefined {
        // skip a replica without a position
        if (record.epoch === null || record.sequence === null) {
            return undefined;
        }

        // take a relayed replica's origin, or the source position
        const position =
            record.originEpoch === null || record.originSequence === null
                ? { epoch: record.epoch, sequence: record.sequence }
                : { epoch: record.originEpoch, sequence: record.originSequence };

        return { position, confirmedAt: record.confirmedAt };
    }

    /** Match the replica's rows of a bookkeeping table keyed by name and scope. */
    #match(
        table:
            | typeof replica
            | typeof replicaPosition
            | typeof replicaPage
            | typeof replicaResult
            | typeof replicaRow,
    ): SQL {
        return and(eq(table.name, this.name), eq(table.scope, this.scope));
    }

    /** Read a batch of the keys the replica includes in a table. */
    async #keys(database: DatabaseConnection, table: Table): Promise<string[]> {
        const rows = await database
            .select({ key: replicaRow.key })
            .from(replicaRow)
            .where(and(this.#match(replicaRow), eq(replicaRow.table, table[TABLE].sqlName)))
            .limit(PRUNE_BATCH);

        return rows.map((row) => row.key);
    }

    /** Collect a replicated row's change into its table's batch. */
    static async #collectBatched(
        batches: Map<Table, Batch>,
        table: Table,
        change: RowChange,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
        open: ApplyOptions["open"],
    ): Promise<void> {
        // leave a predefined row to the database, which keeps it identical everywhere
        const decoded = Table.decode(table, change.row);
        if (Key.isPredefined(table, decoded)) {
            return;
        }

        // open the row and note its key for a snapshot's retractions
        const row =
            open === undefined || change.operation === "delete"
                ? decoded
                : await open(table, decoded);
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
    static async #writeGroups(
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

    /** Write a subscription as a replica's columns, all absent without one. */
    static subscriptionColumns(subscription: Subscription | undefined) {
        const subject = subscription?.subject;

        return {
            publication: subscription?.publication ?? null,
            below: subscription?.below ?? null,
            subjectPackageId: subject?.packageId ?? null,
            subjectType: subject?.type ?? null,
            subjectScope: subject?.scope ?? null,
            subjectId: subject?.id ?? null,
            subjectRelation: subject?.relation ?? null,
            parameters: subscription?.parameters ?? null,
        };
    }

    /** Read the subscription a replica's columns keep, absent without one. */
    static subscriptionOf(row: Select<typeof replica>): SubscriptionRecord | undefined {
        // keep no subscription for a replica without a publication
        const { publication, below, parameters } = row;
        if (publication === null || below === null || parameters === null) {
            return undefined;
        }

        // read the principal decided for, when one is
        const subject =
            row.subjectPackageId === null ||
            row.subjectType === null ||
            row.subjectScope === null ||
            row.subjectId === null
                ? undefined
                : {
                      packageId: row.subjectPackageId,
                      type: row.subjectType,
                      scope: row.subjectScope,
                      id: row.subjectId,
                      ...(row.subjectRelation === null ? {} : { relation: row.subjectRelation }),
                  };

        return {
            name: row.name,
            publication,
            scope: row.scope,
            below,
            ...(subject === undefined ? {} : { subject }),
            parameters,
        };
    }

    /** Record a subscription without where it resumes. */
    static #subscribed(subscription: Subscription): SubscriptionRecord {
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
    static #predictGroups(
        node: Node,
        groups: Map<string, GroupPrediction>,
        row: Row,
        sign: 1 | -1,
    ): void {
        // find or start the row's group
        const group = node.rowGroup(row);
        const key = canonicalize(group);
        const known = groups.get(key) ?? {
            group,
            values: node.emptyValues(),
            rows: 0,
            averages: {},
        };
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
                known.values[name] = Replica.#addScalar(current, present, sign);
            }
            // average present values through their sum and count
            else if (measure.function === "avg") {
                const average = known.averages[name] ?? { sum: 0, count: 0 };
                const next = {
                    sum: Replica.#addScalar(average.sum, present, sign),
                    count: average.count + sign,
                };
                known.averages[name] = next;
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

    /** Take the predictions a completing page rebases: a snapshot, run, group or table. */
    static async #rebased(
        transaction: DatabaseConnection,
        page: Page,
        run: Stage,
        prediction: Prediction | undefined,
    ): Promise<Prediction | undefined> {
        // skip a replica without predictions
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
    static #storedRows(
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

    /** Clear the columns only a replica showed on rows that other replicas keep. */
    static async #clearShown(
        database: DatabaseConnection,
        table: Table,
        shown: readonly { readonly key: string; readonly concealed: readonly string[] }[],
        others: ReadonlyMap<string, readonly (readonly string[])[]>,
    ): Promise<void> {
        // take the columns every other replica hides and this replica showed
        const cleared = shown.flatMap(({ key, concealed }) => {
            const hidden = others.get(key) ?? [];
            const columns = (hidden[0] ?? []).filter(
                (column) =>
                    !concealed.includes(column) && hidden.every((each) => each.includes(column)),
            );

            return columns.length === 0 ? [] : [{ key: Key.parse(table, key), columns }];
        });

        // write the stored rows back with those columns cleared
        const clearing = new Map(
            cleared.map(({ key, columns }) => [Key.name(table, key), columns]),
        );
        const stored = await Replica.#storedRows(
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
    static #addScalar(sum: Scalar, value: Scalar, sign: 1 | -1): Scalar {
        return typeof sum === "string" || typeof value === "string"
            ? String(BigInt(sum ?? 0) + BigInt(sign) * BigInt(value ?? 0))
            : Number(sum ?? 0) + sign * Number(value);
    }

    /** Read a predicted change's images of a node's rows: before negative, after positive. */
    static #predictedImages(
        node: Node,
        change: RowChange,
    ): { readonly image: Row; readonly sign: -1 | 1 }[] {
        // require an update's before
        const table = node.table[TABLE];
        const row = Table.decode(node.table, change.row);
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
            before = Table.decode(node.table, change.before);
        }
        const after = change.operation === "delete" ? undefined : row;

        return [
            ...(before === undefined ? [] : [{ image: before, sign: -1 as const }]),
            ...(after === undefined ? [] : [{ image: after, sign: 1 as const }]),
        ];
    }

    /** Write the home position a replica's rows reflect: a relaying source's, its own, or none. */
    static #originColumns(
        relayed: Origin | undefined,
        isHome: boolean,
        now: number,
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
            return { originEpoch: null, originSequence: null, confirmedAt: now };
        }
        // keep the recorded position
        else {
            return {};
        }
    }

    /** Merge a replica's registration with its position. */
    static #merge(
        registration: Select<typeof replica>,
        position: Select<typeof replicaPosition>,
    ): ReplicaRecord {
        return { ...position, ...registration };
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
    /** The rows the replica keeps. */
    readonly kept: Row[];
    /** The columns the replica hides on each kept row. */
    readonly hidden: string[][];
    /** The rows the replica lets go. */
    readonly removed: Row[];
}

/** A group's last change in one page. */
interface GroupWrite {
    /** The group's key. */
    readonly key: Key<typeof replicaResult>;
    /** The group's kept row or null for a let go group. */
    readonly row: Insert<typeof replicaResult> | null;
}

/** How a replica registers in a database. */
export interface RegisterOptions {
    /** The subscription the replica follows, absent for a replica without a subscription. */
    readonly subscription?: Subscription;
    /** Whose access the source decides the rows by, absent without a subscription. */
    readonly audience?: PublicationAudience;
    /** The clock the replica confirms its position at. */
    readonly clock: Clock;
}

/** How a replica applies its source's pages. */
export interface ApplyOptions extends RegisterOptions {
    /** The client's predictions to rebase. */
    readonly prediction?: Prediction;
    /** Open the machine-bound values a fenced source sealed to this replica. */
    readonly open?: (table: Table, row: Row) => Promise<Row>;
}

/** The home position of a replica's rows. */
export interface Origin {
    /** The home log position the rows reflect. */
    readonly position: LogPosition;
    /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
    readonly confirmedAt: number;
}

/** The inspection of a replica. */
export interface ReplicaInspection {
    /** The replica's name. */
    readonly name: string;
    /** The scope it replicates. */
    readonly scope: string;
    /** The recorded source position. */
    readonly position?: LogPosition;
    /** The home position, absent when the source is the home. */
    readonly origin?: LogPosition;
    /** Whether its tables have the replicated layout. */
    readonly isLaidOut: boolean;
    /** The subscription it completed a run of, absent for a replica without a subscription. */
    readonly subscription?: SubscriptionRecord;
    /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
    readonly confirmedAt?: number;
    /** The staged pages. */
    readonly staged: number;
    /** The aggregate groups it keeps. */
    readonly results: number;
}
