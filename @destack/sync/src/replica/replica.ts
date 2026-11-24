import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import {
    and,
    asc,
    count,
    eq,
    gt,
    gte,
    inArray,
    Key,
    type Insert,
    lt,
    not,
    TABLE,
    type SQL,
    Condition,
    Namespace,
    Predicate,
    Order,
    Scalar,
    DatabaseError,
    type Row,
    type ColumnValue,
    type Relation,
    Relations,
    Table,
    Change,
    LogPosition,
} from "@destack/db";
import {
    type DatabaseConnection,
    Log,
    type Snapshot,
    type Transaction,
} from "@destack/db/database";
import type { QuerySelection, Query, Item } from "../query/query.ts";
import { SyncError } from "../error/error.ts";
import { schema, canonicalize, found, zip } from "@destack/schema";
import { Page, type ResultChange, type RowChange } from "../query/page.ts";
import {
    PublicationAudience,
    Subscription,
    type SubscriptionRecord,
    type Resumption,
} from "./publication.ts";
import { Subject } from "../call/subject.ts";
import type { AggregateGroup } from "../query/aggregate.ts";
import { Node } from "../query/node.ts";
import type { Prediction } from "../prediction/prediction.ts";
import { EVERYONE } from "../feed/audience.ts";
import { Dataflow } from "../dataflow/dataflow.ts";
import { Filter } from "../dataflow/filter.ts";
import { Run } from "../dataflow/run.ts";
import { Trace, type Group, type Upstream } from "../dataflow/upstream.ts";
import { View } from "../dataflow/view.ts";
import { scopeTable } from "../scope/table.ts";
import { RetryPolicy, Telemetry } from "@destack/environment";
import {
    replica,
    replicaScope,
    replicaPosition,
    replicaSubscription,
    replicaPage,
    replicaResult,
    replicaKey,
} from "./table.ts";
import {
    POSITIONED,
    ReplicaRecord,
    ReplicaSet,
    type Layouts,
    type ReplicaPosition,
} from "./set.ts";

/** The staged pages a run's completion reads at once. */
const STAGED_BATCH = 16;

/** The retained keys a snapshot's completion reads at once. */
const PRUNE_BATCH = 1000;

/** The lags a stream goes without a page before its follow ends it: two, past a late beat. */
const SILENT_LAGS = 2;

/** How a follow retries a source lapsing fresh credentials before any page: 5 tries, about 15 s. */
const LAPSE_RETRY = RetryPolicy.of({ maximumAttempts: 5 });

/** How a follow waits out its issuer refusing the follower: from 1 s, at most 100 s apart. */
const REFUSAL_RETRY = RetryPolicy.of();

/** How a follow waits out a source it lost before any page: from 1 s, at most 100 s apart. */
const LOSS_RETRY = RetryPolicy.of();

/** A group's values as a retained group's canonical JSON text has them. */
const GroupValues = schema.record(schema.string(), Scalar);

/** How far an apply got in its stream: its staged pages, its record and the position it reached. */
interface ApplyProgress {
    /** The pages staged of the run so far. */
    readonly staged: number;
    /** Whether the staged run is a snapshot. */
    readonly isSnapshot: boolean;
    /** The parameters the stream reflects from its last reshaped page on. */
    readonly parameters: Page["parameters"];
    /** The replica's record, absent before its first applied run. */
    readonly record: ReplicaRecord | undefined;
    /** The position the stream reached in memory, absent before its first applied run. */
    readonly reached: ReplicaPosition | undefined;
    /** The marks of the last applied page. */
    readonly marks: string | undefined;
}

/** A local replica of one scope's rows in a remote database's tables. */
export class Replica {
    /** The replica's name. */
    readonly name: string;
    /** The scope whose rows the replica stores. */
    readonly scope: string;
    /** The tables the replica lists, parents first, without those only its includes reach. */
    readonly roots: readonly Table[];
    /** The replicated tables: the roots and the tables their includes reach, parents first. */
    readonly tables: readonly Table[];
    /** The condition on each table's replicated rows. */
    readonly where: ReadonlyMap<Table, Condition>;
    /** The scopes of each table whose rows live outside the replica's scope, or in several. */
    readonly scopes: ReadonlyMap<Table, readonly string[]>;
    /** The tables whose rows are replicated from each row's scope. */
    readonly everywhere: ReadonlySet<Table>;
    /** The parent tables of each cross-scope table: each row lives in a parent row's scope. */
    readonly within: ReadonlyMap<Table, readonly Table[]>;
    /** The rows each table's rows include within the replica's scopes, nested as query includes. */
    readonly includes: ReadonlyMap<Table, NonNullable<QuerySelection["with"]>>;
    /** The relations of each table the includes follow, by name. */
    readonly relations: ReadonlyMap<Table, Readonly<Record<string, Relation>>>;
    /** Whether the source may store a replica itself, which the replica follows to its home. */
    readonly isRelayed: boolean;
    /** The scopes the replica reads beyond its scope, sorted. */
    readonly reads: readonly string[];
    /** The replicated tables, by SQL name. */
    readonly #replicated: ReadonlyMap<string, Table>;
    /** The layout digest of each replicated table, by SQL name, absent before its first read. */
    #layoutDigests: Promise<Layouts> | undefined;

    /** Define a replica of a scope's rows in some tables. */
    constructor(definition: {
        /** The replica's name. */
        readonly name: string;
        /** The scope whose rows the replica stores. */
        readonly scope: string;
        /** The tables the replica lists, parents first. */
        readonly tables: readonly Table[];
        /** The condition on each table's replicated rows, none by default. */
        readonly where?: ReadonlyMap<Table, Condition>;
        /** The scopes of each table with rows outside the replica's scope, none by default. */
        readonly scopes?: ReadonlyMap<Table, readonly string[]>;
        /** The tables whose rows are replicated from each row's scope, none by default. */
        readonly everywhere?: ReadonlySet<Table>;
        /** The parent tables of each table replicated across scopes, none by default. */
        readonly within?: ReadonlyMap<Table, readonly Table[]>;
        /** The rows each table's rows include, none by default. */
        readonly includes?: ReadonlyMap<Table, NonNullable<QuerySelection["with"]>>;
        /** The relations of each table the includes follow, none by default. */
        readonly relations?: ReadonlyMap<Table, Readonly<Record<string, Relation>>>;
        /** Whether the source may store a replica itself, true by default. */
        readonly isRelayed?: boolean;
    }) {
        // store the identity
        this.name = definition.name;
        this.scope = definition.scope;
        this.where = definition.where ?? new Map();
        this.scopes = definition.scopes ?? new Map();
        this.everywhere = definition.everywhere ?? new Set();
        this.within = definition.within ?? new Map();
        this.includes = definition.includes ?? new Map();
        this.relations = definition.relations ?? new Map();
        this.isRelayed = definition.isRelayed ?? true;

        // hold the queried tables and every table their includes reach
        const relations = new Relations(this.relations);
        const reached = [...this.includes].flatMap(([table, included]) =>
            Node.tables({ table, scopes: [this.scope], with: included, relations }),
        );
        this.roots = definition.tables;
        this.tables = [...new Set([...definition.tables, ...reached])];
        this.#replicated = new Map(this.tables.map((table) => [table[TABLE].sqlName, table]));

        // list the scopes the replica reads beyond its scope
        const reads = [...new Set([...this.scopes.values()].flat())]
            .filter((scope) => scope !== this.scope)
            .toSorted();
        this.reads = reads;
    }

    /** Decide whether the replica holds a scope's rows: its own scope, or one it reads. */
    holds(scope: string): boolean {
        return scope === this.scope || this.reads.includes(scope);
    }

    /** Join replicas of several scopes into one replica of a scope. */
    static union(name: string, scope: string, replicas: readonly Replica[]): Replica {
        // list the tables in the order the replicas list them, parents first
        const tables = [...new Set(replicas.flatMap((each) => each.roots))];
        const where = new Map<Table, Condition>();
        const scopes = new Map<Table, readonly string[]>();
        const everywhere = new Set<Table>();
        for (const table of tables) {
            // read a table as its replicas read it, each arm within its own scopes unless it reads every scope
            const reading = replicas.filter((each) => each.roots.includes(table));
            const arms = reading.map((each): Condition | undefined => {
                const condition = each.where.get(table);

                return each.everywhere.has(table) || reading.length === 1
                    ? condition
                    : {
                          AND: [
                              Node.scoped(each.scopes.get(table) ?? [each.scope]),
                              condition ?? {},
                          ],
                      };
            });
            const [only] = arms;
            if (arms.length === 1 && only !== undefined) {
                where.set(table, only);
            } else if (arms.length > 1) {
                where.set(table, { OR: arms.map((arm) => arm ?? {}) });
            }

            // read every scope once any replica does, and record the scopes the rows live in
            if (reading.some((each) => each.everywhere.has(table))) {
                everywhere.add(table);
            }
            scopes.set(table, [
                ...new Set(reading.flatMap((each) => each.scopes.get(table) ?? [each.scope])),
            ]);
        }

        // keep the parents, includes and relations of every replica
        const within = new Map(replicas.flatMap((each) => [...each.within]));
        const includes = new Map<Table, NonNullable<QuerySelection["with"]>>();
        const relations = new Map<Table, Readonly<Record<string, Relation>>>();
        for (const each of replicas) {
            for (const [table, included] of each.includes) {
                includes.set(table, { ...includes.get(table), ...included });
            }
            for (const [table, named] of each.relations) {
                relations.set(table, { ...relations.get(table), ...named });
            }
        }

        return new Replica({
            name,
            scope,
            tables,
            where,
            scopes,
            everywhere,
            within,
            includes,
            relations,
            isRelayed: replicas.every((each) => each.isRelayed),
        });
    }

    /** The queries a fenced source's capture reads: every replicated table, unlogged ones too. */
    get captured(): Record<string, Query> {
        // refuse tables replicated across scopes
        if (this.within.size > 0) {
            throw SyncError.of(
                "SCOPE_INVALID",
                `replica '${this.name}' is invalid: it replicates tables across scopes, and no capture can read them`,
            );
        }
        // refuse tables held through includes
        else if (this.tables.length > this.roots.length) {
            throw SyncError.of(
                "SCOPE_INVALID",
                `replica '${this.name}' is invalid: it includes rows of other tables, and no capture can read them`,
            );
        }

        return Object.fromEntries(
            this.tables.map((table) => [table[TABLE].sqlName, this.#query(table)]),
        );
    }

    /** The queries the replica follows by table: its logged tables and the source's record. */
    get queries(): Record<string, Query> {
        // follow each queried logged table outside a parent's rows
        const queries = this.roots
            .filter((table) => !this.within.has(table) && table[TABLE].retention !== "none")
            .map((table): [string, Query] => {
                // include the parent's children and the declared includes
                const included = this.includes.get(table);
                const nested = this.#children(table).with;
                const selection =
                    included === undefined && nested === undefined
                        ? {}
                        : { with: { ...nested, ...included } };

                return [
                    table[TABLE].sqlName,
                    { ...this.#query(table), ...selection, relations: this.#relate() },
                ];
            });

        // follow the source's replica position when relayed
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

    /** Relate the replicated tables to their scopes' tables and to the included ones. */
    #relate(): Relations {
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

        // add the relations the includes follow
        for (const [table, named] of this.relations) {
            relations.set(table, { ...relations.get(table), ...named });
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

    /** Wait until the replica reflects its home up to a position, or the signal aborts. */
    async wait(
        database: DatabaseConnection,
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<boolean> {
        return await new ReplicaSet(database).waitAt([this], position, signal);
    }

    /** Report whether the database stores the replica's record. */
    async isRecorded(snapshot: Snapshot): Promise<boolean> {
        return (
            (await snapshot
                .read(replica)
                .findFirst({ where: { name: this.name, scope: this.scope } })) !== undefined
        );
    }

    /** Refuse publishing the replica to a subscriber whose rows the database replicates into it. */
    async requireAcyclic(database: DatabaseConnection, origin: string): Promise<void> {
        // read the database's replicas with the tables each replicates
        const records = await database
            .select({
                name: replica.name,
                scope: replica.scope,
                log: replicaPosition.log,
                originLog: replicaPosition.originLog,
                tables: replica.tables,
            })
            .from(replica)
            .innerJoin(replicaPosition, POSITIONED);
        // refuse a replica from the subscriber storing one of the replica's tables, with rows or without yet
        const [cyclic] = records.flatMap((record) => {
            const shared = record.tables.find((table) => this.#replicated.has(table));
            const isFromOrigin = record.log === origin || record.originLog === origin;

            return shared !== undefined && isFromOrigin ? [{ record, shared }] : [];
        });
        if (cyclic !== undefined) {
            const table = found(this.#replicated, cyclic.shared)[TABLE].name;
            throw SyncError.of(
                "FOLLOW_BLOCKED",
                `cannot follow replica '${this.name}' of '${this.scope}': it would publish '${table}' back to '${origin}', its source in replica '${cyclic.record.name}' of '${cyclic.record.scope}'`,
            );
        }
    }

    /** Read the replica's source position, absent before a snapshot or after a layout change. */
    async position(database: DatabaseConnection): Promise<LogPosition | undefined> {
        return this.#recordedPosition(await this.#record(database), await this.#layouts(database));
    }

    /** List the scopes each replicated table's rows live in, by SQL name. */
    #living(): Record<string, string[]> {
        return Object.fromEntries(
            this.tables.map((table) => [
                table[TABLE].sqlName,
                [...(this.scopes.get(table) ?? [this.scope])],
            ]),
        );
    }

    /** Write the scopes the replica's tables' rows live in, replacing rows that say otherwise. */
    async #writeLiving(
        transaction: DatabaseConnection,
        record: Pick<ReplicaRecord, "living"> | undefined,
    ): Promise<void> {
        // leave rows that say the same
        const living = ReplicaSet.sortedLiving(this.#living());
        const stored = record?.living ?? {};
        if (canonicalize(stored) === canonicalize(living)) {
            return;
        }

        // replace a recorded replica's rows by a row per table and living scope
        const key = { name: this.name, scope: this.scope };
        const rows = Object.entries(living).flatMap(([table, scopes]) =>
            scopes.map((scope) => ({ ...key, table, living: scope })),
        );
        if (Object.keys(stored).length > 0) {
            await transaction.delete(replicaScope).where(this.#match(replicaScope));
        }
        if (rows.length > 0) {
            await transaction.insert(replicaScope).values(rows).onConflictDoNothing();
        }
    }

    /** Digest each replicated table's layout on first read, with a database's WebCrypto. */
    #layouts(database: DatabaseConnection): Promise<Layouts> {
        const { crypto } = database.state.environment;
        this.#layoutDigests ??= Promise.all(
            this.tables.map(
                async (table) => [table[TABLE].sqlName, await Log.layout(table, crypto)] as const,
            ),
        ).then((digests) => Object.fromEntries(digests));

        return this.#layoutDigests;
    }

    /** Read the replica's record, absent before it registered. */
    async #record(database: DatabaseConnection): Promise<ReplicaRecord | undefined> {
        return new ReplicaSet(database).find(this.name, this.scope);
    }

    /** Report whether a record registers what a page follows: subscription, layout and scopes. */
    #isRegistered(
        record: ReplicaRecord | undefined,
        layouts: Layouts,
        page: Page,
        options: ApplyOptions,
    ): boolean {
        // require a record, with the page's scopes and subjects when it moves them
        if (
            record === undefined ||
            (page.scopes !== undefined &&
                canonicalize(record.scopes) !== canonicalize(page.scopes)) ||
            (page.subjects !== undefined &&
                canonicalize(record.subjects) !== canonicalize(page.subjects))
        ) {
            return false;
        }

        // require the subscription, or none, decided for the same audience
        const { subscription, audience } = options;
        const isSubscribed =
            subscription === undefined
                ? record.subscription === undefined
                : record.audience === audience &&
                  canonicalize(record.subscription ?? null) ===
                      canonicalize(ReplicaSet.subscribed(subscription));

        return (
            isSubscribed &&
            canonicalize(record.layouts) === canonicalize(layouts) &&
            canonicalize(record.tables) === canonicalize([...this.#replicated.keys()]) &&
            canonicalize(record.living) === canonicalize(ReplicaSet.sortedLiving(this.#living()))
        );
    }

    /** Build the record a completed run leaves: its registration, position and subscription. */
    #recorded(
        record: ReplicaRecord | undefined,
        page: Page,
        position: ReplicaPosition,
        layouts: Layouts,
        options: ApplyOptions,
    ): ReplicaRecord {
        const { subscription, audience } = options;

        return {
            name: this.name,
            scope: this.scope,
            scopes: page.scopes ?? record?.scopes ?? null,
            subjects: page.subjects ?? record?.subjects ?? null,
            layouts,
            tables: [...this.#replicated.keys()],
            living: this.#living(),
            ...position,
            subscription:
                subscription === undefined
                    ? record?.subscription
                    : ReplicaSet.subscribed(subscription),
            audience: subscription === undefined ? record?.audience : audience,
        };
    }

    /** Read a record's source position, absent before a snapshot or after a kept table's change. */
    #recordedPosition(
        record: Pick<ReplicaRecord, "log" | "epoch" | "sequence" | "layouts"> | undefined,
        layouts: Layouts,
    ): LogPosition | undefined {
        return record === undefined ||
            record.log === null ||
            record.epoch === null ||
            record.sequence === null ||
            !ReplicaSet.isLaidOut(record.layouts, layouts)
            ? undefined
            : { log: record.log, epoch: record.epoch, sequence: record.sequence };
    }

    /** Describe the replica's position, layout, queries and staged pages. */
    async inspect(database: DatabaseConnection): Promise<ReplicaInspection> {
        // read the record, its subscription, staged pages and groups
        const row = await this.#record(database);
        const subscription = row?.subscription;
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
            ...(row === undefined ? {} : ReplicaRecord.positions(row)),
            isLaidOut: ReplicaSet.isLaidOut(row?.layouts ?? null, await this.#layouts(database)),
            ...(subscription === undefined ? {} : { subscription }),
            staged: staged.pages,
            results: results.groups,
        };
    }

    /** Read the subscription of the replica's last completed run, if any. */
    async subscribed(database: DatabaseConnection): Promise<SubscriptionRecord | undefined> {
        return (await this.#record(database))?.subscription;
    }

    /** Resume a subscription from the replica: from its position, as a reshape, or by snapshot. */
    async resume(
        database: DatabaseConnection,
        subscription?: SubscriptionRecord,
    ): Promise<Resumption> {
        // read the recorded position, the subscription it reflects and whether the replica gained tables since
        const record = await this.#record(database);
        const after = this.#recordedPosition(record, await this.#layouts(database));
        const reflected = record?.subscription;
        const isGrown = this.tables.some(
            (table) => !(record?.tables.includes(table[TABLE].sqlName) ?? false),
        );
        const origin = (await database.log.position()).log;
        const subscribed =
            subscription === undefined ? undefined : ReplicaSet.subscribed(subscription);

        // compare the subscription with the reflected one apart from its parameters
        const isOther =
            subscribed !== undefined &&
            (reflected === undefined ||
                canonicalize({ ...reflected, parameters: null }) !==
                    canonicalize({ ...subscribed, parameters: null }));

        // snapshot without a position, or for another subscription than the reflected one
        if (after === undefined || isOther) {
            return { origin };
        }
        // reshape from the reflected parameters, whose source sends what the subscribed ones add
        else if (
            subscribed !== undefined &&
            reflected !== undefined &&
            canonicalize(reflected.parameters) !== canonicalize(subscribed.parameters)
        ) {
            return { after, previous: reflected.parameters, origin };
        }
        // resume from the position, unless the replica gained tables its parameters never sent
        else {
            return isGrown ? { origin } : { after, origin };
        }
    }

    /** Read the replica's scope chain, nearest first: only its scope until its source sends it. */
    async chain(database: DatabaseConnection): Promise<readonly string[]> {
        const record = await this.#record(database);

        return record?.scopes ?? [this.scope];
    }

    /** Read the subjects the replica's rows are decided for: none until its source sends them. */
    async subjects(database: DatabaseConnection): Promise<readonly Subject[]> {
        const record = await this.#record(database);

        return record?.subjects ?? [];
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
    ): Promise<AggregateGroup[]> {
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
                    throw new TypeError("a retained group is missing its query or values");
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
        ).filter((change) => change.table[TABLE].sqlName === table);
        const images = predicted.flatMap(({ before, after }) => [
            ...(before === null ? [] : [{ image: before, sign: -1 as const }]),
            ...(after === null ? [] : [{ image: after, sign: 1 as const }]),
        ]);
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

    /** Turn the replicated rows into the database's local rows. */
    async promote(transaction: Transaction): Promise<void> {
        // require a complete replica
        const key = { name: this.name, scope: this.scope };
        const registration = await transaction.read(replica).findFirst({ where: key });
        const position = await transaction.read(replicaPosition).findFirst({ where: key });
        const layouts = await this.#layouts(transaction.database);
        if (
            registration === undefined ||
            position === undefined ||
            this.#recordedPosition({ ...position, layouts: registration.layouts }, layouts) ===
                undefined
        ) {
            throw SyncError.of(
                "REPLICA_UNAVAILABLE",
                `replica of '${this.scope}' is unavailable: it has no position yet`,
            );
        }

        // drop the replica's records now
        const subscription = await transaction.read(replicaSubscription).findFirst({ where: key });
        await transaction.write({ table: replica, before: registration, after: null });
        if (subscription !== undefined) {
            await transaction.write({
                table: replicaSubscription,
                before: subscription,
                after: null,
            });
        }
        await transaction.write({ table: replicaPosition, before: position, after: null });

        // drop the rows of what it stores, so the transaction finds its scopes copied by no replica
        for (const row of await transaction.read(replicaScope).findMany({ where: key })) {
            await transaction.write({ table: replicaScope, before: row, after: null });
        }

        // drop its keys of copied scope rows in this transaction
        const scopes = scopeTable[TABLE].sqlName;
        const copied = await transaction
            .read(replicaKey)
            .findMany({ where: { ...key, table: scopes } });
        for (const row of copied) {
            await transaction.write({ table: replicaKey, before: row, after: null });
        }

        // drop its other keys with its pages and results on commit
        transaction.record(
            async (database) =>
                await database.transaction(async (dropping) => {
                    await dropping.delete(replicaPage).where(this.#match(replicaPage));
                    await dropping.delete(replicaResult).where(this.#match(replicaResult));
                    await dropping
                        .delete(replicaKey)
                        .where(and(this.#match(replicaKey), not(eq(replicaKey.table, scopes))));
                }),
        );
    }

    /** Record the replica before its first snapshot, with the subscription it follows, if any. */
    async register(database: DatabaseConnection, options: RegisterOptions = {}): Promise<void> {
        // leave a replica recorded already with a subscription, or without one to record
        const { subscription, audience } = options;
        const record = await this.#record(database);
        if (
            record !== undefined &&
            (subscription === undefined || record.subscription !== undefined)
        ) {
            return;
        }

        // record it, its subscription and its unset position in one transaction
        await database.transaction(async (transaction) => {
            // record the replica, and the subscription it follows if any
            const key = { name: this.name, scope: this.scope };
            await transaction
                .insert(replica)
                .values({ ...key, tables: [...this.#replicated.keys()] })
                .onConflictDoNothing();
            if (record === undefined) {
                await this.#writeLiving(transaction, undefined);
            }
            if (subscription !== undefined) {
                await transaction
                    .insert(replicaSubscription)
                    .values({ ...key, ...ReplicaSet.subscriptionColumns(subscription, audience) })
                    .onConflictDoNothing();
            }
            const unpositioned = {
                ...key,
                log: null,
                epoch: null,
                sequence: null,
            };
            await transaction.insert(replicaPosition).values(unpositioned).onConflictDoNothing();
        });
    }

    /** Drop a replica: delete the unshared rows of its tables and every recorded one, forget it. */
    static async drop(
        database: DatabaseConnection,
        { name, scope, tables }: Pick<Replica, "name" | "scope" | "tables">,
    ): Promise<void> {
        const dropped = new Replica({ name, scope, tables });
        await database.state.serialize(() =>
            database.transaction(
                async (transaction) => {
                    await transaction.log.asReplica(async () => {
                        // take every included row out, those of recorded tables it stopped replicating first
                        const record = await dropped.#record(transaction);
                        await dropped.#excludeAll(
                            transaction,
                            dropped.#retired(transaction, record),
                        );
                        await dropped.#excludeAll(transaction, tables);

                        // forget the groups, staged pages and record
                        await transaction
                            .delete(replicaResult)
                            .where(dropped.#match(replicaResult));
                        await transaction.delete(replicaPage).where(dropped.#match(replicaPage));
                        await transaction
                            .delete(replicaPosition)
                            .where(dropped.#match(replicaPosition));
                        await transaction
                            .delete(replicaSubscription)
                            .where(dropped.#match(replicaSubscription));
                        await transaction.delete(replicaScope).where(dropped.#match(replicaScope));
                        await transaction.delete(replica).where(dropped.#match(replica));
                    });
                },
                { constraints: "deferred" },
            ),
        );

        // forget the dropped rows' mark, so nothing reads the replica live until it is followed again
        database.tracker.unmark(ReplicaSet.owner(name, scope));
    }

    /** Receive one page of a stream: stage it, or apply the run it completes. */
    async #receive(
        database: DatabaseConnection,
        page: Page,
        options: ApplyOptions,
        progress: ApplyProgress,
    ): Promise<ApplyProgress> {
        // take the apply's progress so far
        let { staged, isSnapshot, parameters, record, reached, marks } = progress;
        const attributed = {
            "destack.replica.name": this.name,
            "destack.replica.scope": this.scope,
        };

        // register the parameters a reshaped stream reflects from this page on
        parameters = page.parameters ?? parameters;
        const reflected: ApplyOptions =
            options.subscription === undefined || parameters === undefined
                ? options
                : { ...options, subscription: { ...options.subscription, parameters } };

        // drop staged pages a snapshot replaces
        if (page.reset && staged > 0) {
            await database.transaction(
                async (transaction) =>
                    await transaction.delete(replicaPage).where(this.#match(replicaPage)),
            );
            staged = 0;
        }
        if (staged === 0) {
            isSnapshot = page.reset;
        }

        // apply the run a page completes in its span, timed by whether it is a snapshot
        if (page.complete) {
            const run = { staged, isSnapshot };
            const kind = { "destack.replica.snapshot": isSnapshot };
            const recorded = record;
            ({ position: reached, record } = await this.#scope(database).span(
                "destack.replica.apply",
                { ...attributed, ...kind },
                () => this.#complete(database, page, run, recorded, reflected),
                { duration: { attributes: kind } },
            ));

            // announce the run's home position at once
            const home = this.#home(reached);
            if (home !== undefined) {
                new ReplicaSet(database).mark(this, home);
            }
            marks = Replica.#marks(page);
            staged = 0;
        }
        // stage a page within a run, its first page dropping an earlier stream's
        else {
            const order = staged;
            await this.#scope(database).span("destack.replica.stage", attributed, () =>
                database.transaction(async (transaction) => {
                    if (order === 0) {
                        await transaction.delete(replicaPage).where(this.#match(replicaPage));
                    }
                    await transaction
                        .insert(replicaPage)
                        .values({ name: this.name, scope: this.scope, index: order, page });
                }),
            );
            staged += 1;
        }

        return { staged, isSnapshot, parameters, record, reached, marks };
    }

    /** Apply one stream of pages, staging each run until it completes. */
    async *apply(
        database: DatabaseConnection,
        pages: AsyncIterable<Page> | Iterable<Page>,
        options: ApplyOptions = {},
    ): AsyncGenerator<Page> {
        // start at the record, staging nothing yet
        const { telemetry } = database.state.environment;
        let progress: ApplyProgress = {
            staged: 0,
            isSnapshot: false,
            parameters: options.subscription?.parameters,
            record: await this.#record(database),
            reached: undefined,
            marks: undefined,
        };
        const attributed = {
            "destack.replica.name": this.name,
            "destack.replica.scope": this.scope,
        };
        for await (const page of pages) {
            // move past a bare page continuing this stream's run in memory
            const received = progress;
            const { staged, reached, marks } = received;
            if (
                page.complete &&
                reached !== undefined &&
                Replica.#isBeat(page, staged, reached, marks, options)
            ) {
                const before = this.#home(reached);
                const beaten = Replica.#beat(page, reached);
                const home = this.#home(beaten);
                progress = { ...received, reached: beaten };

                // refresh an unmoved home position with the next heartbeat
                if (home !== undefined && before !== undefined && LogPosition.same(home, before)) {
                    new ReplicaSet(database).refresh(this, home);
                }
                // announce a moved one at once
                else if (home !== undefined) {
                    new ReplicaSet(database).mark(this, home);
                }
            }
            // receive any other page in a span of its own, linked to the span following the source
            else {
                progress = await Telemetry.detach(telemetry, (links) =>
                    this.#scope(database).span(
                        "destack.replica.receive",
                        attributed,
                        () => this.#receive(database, page, options, received),
                        { links },
                    ),
                );
            }
            yield page;
        }
    }

    /** Apply a source's pages until aborted, resuming each stream ending where the replica is. */
    async follow(
        database: DatabaseConnection,
        source: (
            from: Resumption,
            signal: AbortSignal,
            subscription: Subscription | undefined,
        ) => AsyncIterable<Page>,
        signal: AbortSignal,
        options: ApplyOptions<Subscription> & {
            /** Report the replica caught up, once per follow at its first full page. */
            readonly synced: () => void;
            /** Take each applied page, such as for the events it brings. */
            readonly received?: (page: Page) => Promise<void>;
            /** End a stream silent for twice this lag, in milliseconds, to resume it. */
            readonly lag?: number;
        },
    ): Promise<void> {
        // apply each stream from where the replica is until aborted, ending a silent one to resume
        await this.register(database, options);
        let silent = new AbortController();
        const ended = new AbortController();
        const silence = this.#silence(
            database,
            options.lag,
            AbortSignal.any([signal, ended.signal]),
            () => silent.abort(),
        );
        let isSynced = false;
        let lapses = 0;
        let refusals = 0;
        let losses = 0;
        try {
            while (!signal.aborted) {
                let isReceived = false;
                let isLapsed = false;
                let isRefused = false;
                let isLost = false;
                let lost = "";
                silent = new AbortController();
                const streaming = AbortSignal.any([signal, silent.signal]);

                // take the subscription once, so the stream opens and records the same parameters
                const { subscription } = options;
                const { telemetry } = database.state.environment;
                const from = await Telemetry.detach(telemetry, (links) =>
                    this.#scope(database).span(
                        "destack.replica.resume",
                        { "destack.replica.name": this.name, "destack.replica.scope": this.scope },
                        () => this.resume(database, subscription),
                        { links },
                    ),
                );
                const pages = source(from, streaming, subscription);
                const opened = subscription === undefined ? options : { ...options, subscription };
                silence();
                try {
                    for await (const page of this.apply(database, pages, opened)) {
                        // report the replica synced at its first complete page
                        isReceived = true;
                        silence();
                        await options.received?.(page);
                        if (page.complete && !isSynced) {
                            isSynced = true;
                            options.synced();
                        }
                    }
                } catch (error) {
                    // resume with a fresh credential once the source refused the lapsed one
                    isLapsed = SyncError.is(error, "CREDENTIAL_UNAUTHENTICATED");

                    // wait out a follower its issuer no longer authenticates, as an operator may restore it
                    isRefused = SyncError.is(error, "FOLLOWER_UNAUTHENTICATED");

                    // resume from the replica's position once the connection to the source dropped
                    isLost = SyncError.is(error, "CONNECTION_UNAVAILABLE");
                    lost = isLost && error instanceof Error ? error.message : "";

                    // end quietly on this follow's own abort or a silent stream's, failing on anything else
                    const isAborted = signal.aborted && error === signal.reason;
                    const isSilent = silent.signal.aborted && !signal.aborted;
                    if (!isLapsed && !isRefused && !isLost && !isAborted && !isSilent) {
                        throw error;
                    }
                }

                // resume a stream ended for its silence like one that delivered pages, warning of it
                const isSilenced = silent.signal.aborted && !signal.aborted;
                if (isSilenced) {
                    this.#scope(database).log.warn("destack.replica.silent", {
                        "destack.replica.name": this.name,
                        "destack.replica.scope": this.scope,
                        "destack.replica.received": isReceived,
                    });
                }
                isReceived ||= isSilenced;

                // back off before following again as a follower its issuer refuses, logging the first refusal
                if (isRefused) {
                    refusals += 1;
                    if (refusals === 1) {
                        this.#scope(database).log.warn("destack.replica.refused", {
                            "destack.replica.name": this.name,
                            "destack.replica.scope": this.scope,
                        });
                    }
                    const { clock } = database.state;
                    const drawn = RetryPolicy.draw(
                        `${this.name} ${this.scope} refused ${refusals}`,
                    );
                    await RetryPolicy.pause(REFUSAL_RETRY, refusals, clock, signal, () => drawn);
                }
                // back off before resuming a source lost before any page, logging the first loss
                else if (!isReceived && isLost) {
                    losses += 1;
                    if (losses === 1) {
                        this.#scope(database).log.warn("destack.replica.lost", {
                            "destack.replica.name": this.name,
                            "destack.replica.scope": this.scope,
                            "error.message": lost,
                        });
                    }
                    const { clock } = database.state;
                    const drawn = RetryPolicy.draw(`${this.name} ${this.scope} lost ${losses}`);
                    await RetryPolicy.pause(LOSS_RETRY, losses, clock, signal, () => drawn);
                }
                // reject a stream that ends without a page, other than one whose credential lapsed
                else if (!isReceived && !isLapsed && !signal.aborted) {
                    throw SyncError.of(
                        "STREAM_UNREACHABLE",
                        `could not read a page from the source of '${this.scope}' before the stream ended`,
                    );
                }
                // fail once the source lapses every fresh credential
                else if (
                    !isReceived &&
                    isLapsed &&
                    !RetryPolicy.isRetried(LAPSE_RETRY, lapses + 1)
                ) {
                    throw SyncError.of(
                        "CREDENTIAL_UNAUTHENTICATED",
                        `authentication required: the source of '${this.scope}' lapsed fresh credentials before any page after ${lapses + 1} attempts`,
                    );
                }
                // back off before resuming a stream whose fresh credential lapsed before any page
                else if (!isReceived && isLapsed) {
                    lapses += 1;
                    const { clock } = database.state;
                    const drawn = RetryPolicy.draw(`${this.name} ${this.scope} ${lapses}`);
                    await RetryPolicy.pause(LAPSE_RETRY, lapses, clock, signal, () => drawn);
                }
                // resume at once after a stream that delivered pages, logging a follower restored
                else {
                    if (refusals > 0 || losses > 0) {
                        this.#scope(database).log.info("destack.replica.restored", {
                            "destack.replica.name": this.name,
                            "destack.replica.scope": this.scope,
                        });
                    }
                    lapses = 0;
                    refusals = 0;
                    losses = 0;
                }
            }
        } finally {
            // stop the silence timer when the follow ends, also when it fails
            ended.abort();
        }
    }

    /** Read the replica's telemetry scope from its database's environment. */
    #scope(database: DatabaseConnection): TelemetryScope {
        return TelemetryScope.of(import.meta.destack.package, database.state.environment.telemetry);
    }

    /** Arm the silence ending a stream without a page for two lags, rearmed at each page. */
    #silence(
        database: DatabaseConnection,
        lag: number | undefined,
        signal: AbortSignal,
        silent: () => void,
    ): () => void {
        // end nothing without a lag
        if (lag === undefined) {
            return () => {};
        }

        // replace the armed silence at each stream start and page
        const { clock } = database.state;
        let waking = new AbortController();

        return () => {
            waking.abort();
            waking = new AbortController();
            clock.sleep(lag * SILENT_LAGS, AbortSignal.any([signal, waking.signal])).then(
                () => silent(),
                () => {},
            );
        };
    }

    /** Apply a run's pages in one transaction around reverted predictions, answering its record. */
    async #complete(
        database: DatabaseConnection,
        page: Page,
        run: Stage,
        record: ReplicaRecord | undefined,
        options: ApplyOptions,
    ): Promise<{ readonly position: ReplicaPosition; readonly record: ReplicaRecord }> {
        // apply after the database's earlier serialized work
        return await database.state.serialize(() =>
            database.transaction(
                async (transaction) => {
                    // rebase onto pages that touch a prediction
                    const rebased = await Replica.#rebased(
                        transaction,
                        page,
                        run,
                        options.prediction,
                    );

                    // require the recorded epoch outside a snapshot
                    const layouts = await this.#layouts(transaction);
                    const recorded = this.#recordedPosition(record, layouts);
                    const isContinued =
                        recorded?.log === page.position.log &&
                        recorded.epoch === page.position.epoch;
                    if (!run.isSnapshot && recorded !== undefined && !isContinued) {
                        throw DatabaseError.of(
                            "EPOCH_CONFLICT",
                            `cannot apply page of log '${page.position.log}' epoch ${page.position.epoch}: it does not continue the replica of log '${recorded.log}' epoch ${recorded.epoch}`,
                        );
                    }

                    // write a run's rows as a replica after reverting predictions
                    const retired = this.#retired(transaction, record);
                    const isRegistered = this.#isRegistered(record, layouts, page, options);
                    const isWritten =
                        rebased !== undefined ||
                        run.isSnapshot ||
                        run.staged > 0 ||
                        page.changes.length > 0 ||
                        (page.results ?? []).length > 0 ||
                        retired.length > 0 ||
                        !isRegistered;
                    const relayed = isWritten
                        ? await transaction.log.asReplica(async () => {
                              await rebased?.revert(transaction);

                              return await this.#writeRun(transaction, page, run, record, options);
                          })
                        : undefined;

                    // record the position of a run without rows, unless the record has it
                    const origin = page.origin ?? relayed;
                    const position = Replica.#position(page, run, record, origin);
                    if (!isWritten && !ReplicaRecord.isAt(record, position)) {
                        await transaction.log.asReplica(() =>
                            this.#writePosition(transaction, position),
                        );
                    }

                    // predict again what the source lacks, at the home position
                    await rebased?.replay(transaction, origin ?? page.position);

                    return {
                        position,
                        record: this.#recorded(record, page, position, layouts, options),
                    };
                },
                { constraints: "deferred" },
            ),
        );
    }

    /** Write a run's pages, registration and position, answering its relayed origin. */
    async #writeRun(
        transaction: DatabaseConnection,
        page: Page,
        run: Stage,
        record: ReplicaRecord | undefined,
        options: ApplyOptions,
    ): Promise<LogPosition | undefined> {
        // let go of every group on a snapshot, and of the rows of tables the replica stopped replicating
        if (run.isSnapshot) {
            await transaction.delete(replicaResult).where(this.#match(replicaResult));
        }
        const retired = this.#retired(transaction, record);
        await this.#excludeAll(transaction, retired);

        // write the staged and completing pages, past the changes of retired tables, and prune what a snapshot left out
        const delivered = run.isSnapshot ? this.#deliveries() : undefined;
        const left = new Set(retired.map((table) => table[TABLE].sqlName));
        const staging = await this.#writeStaged(transaction, run.staged, delivered, left, options);
        const written = (await this.#write(transaction, page, delivered, left, options)) ?? staging;
        const relayed = page.origin ?? written;
        if (delivered !== undefined) {
            await this.#pruneUndelivered(transaction, delivered);
        }

        // register what the run follows and publish the position its rows reflect
        await this.#register(transaction, page, record, options);
        await this.#publish(transaction, page, run, record, relayed);

        return relayed;
    }

    /** Record what a run follows, unless the record has it. */
    async #register(
        transaction: DatabaseConnection,
        page: Page,
        record: ReplicaRecord | undefined,
        options: ApplyOptions,
    ): Promise<void> {
        // read what the replica replicates
        const { subscription, audience } = options;
        const key = { name: this.name, scope: this.scope };
        const registration = {
            ...(page.scopes === undefined ? {} : { scopes: page.scopes }),
            ...(page.subjects === undefined ? {} : { subjects: page.subjects }),
            layouts: await this.#layouts(transaction),
            tables: [...this.#replicated.keys()],
        };

        // register what the run follows unless the record has it
        if (this.#isRegistered(record, registration.layouts, page, options)) {
            return;
        }
        await transaction
            .insert(replica)
            .values({ ...key, ...registration })
            .onConflictDoUpdate({ target: [replica.name, replica.scope], set: registration });
        await this.#writeLiving(transaction, record);
        if (subscription !== undefined) {
            const columns = ReplicaSet.subscriptionColumns(subscription, audience);
            await transaction
                .insert(replicaSubscription)
                .values({ ...key, ...columns })
                .onConflictDoUpdate({
                    target: [replicaSubscription.name, replicaSubscription.scope],
                    set: columns,
                });
        }
    }

    /** Publish the position a run's rows reflect, and drop its staged pages. */
    async #publish(
        transaction: DatabaseConnection,
        page: Page,
        run: Stage,
        record: ReplicaRecord | undefined,
        relayed: LogPosition | undefined,
    ): Promise<void> {
        await this.#writePosition(transaction, Replica.#position(page, run, record, relayed));
        if (run.staged > 0) {
            await transaction.delete(replicaPage).where(this.#match(replicaPage));
        }
    }

    /** Read the position a run reached at its source, with its home position. */
    static #position(
        page: Page,
        run: Stage,
        record: ReplicaRecord | undefined,
        relayed: LogPosition | undefined,
    ): ReplicaPosition {
        return { ...page.position, ...Replica.#originColumns(page, run, record, relayed) };
    }

    /** Move past a bare page's position in memory, taking a relaying source's origin. */
    static #beat(page: Page, reached: ReplicaPosition): ReplicaPosition {
        const { origin } = page;

        return {
            ...reached,
            ...page.position,
            ...(origin === undefined
                ? {}
                : {
                      originLog: origin.log,
                      originEpoch: origin.epoch,
                      originSequence: origin.sequence,
                  }),
        };
    }

    /** Read the home position of a replica position, absent before its first snapshot. */
    #home(at: ReplicaPosition): LogPosition | undefined {
        return ReplicaRecord.origin({ name: this.name, scope: this.scope, ...at });
    }

    /** Write the position the replica reached at its source, and its rows' home position. */
    async #writePosition(database: DatabaseConnection, position: ReplicaPosition): Promise<void> {
        await database
            .insert(replicaPosition)
            .values({ name: this.name, scope: this.scope, ...position })
            .onConflictDoUpdate({
                target: [replicaPosition.name, replicaPosition.scope],
                set: position,
            });
    }

    /** Report whether a page only moves the position of a stream's applied run. */
    static #isBeat(
        page: Page,
        staged: number,
        reached: ReplicaPosition,
        marks: string | undefined,
        options: ApplyOptions,
    ): boolean {
        return (
            !page.reset &&
            staged === 0 &&
            page.changes.length === 0 &&
            (page.results ?? []).length === 0 &&
            (Replica.#marks(page) === marks ||
                (page.scopes === undefined && page.subjects === undefined)) &&
            page.parameters === undefined &&
            options.prediction === undefined &&
            page.position.log === reached.log &&
            page.position.epoch === reached.epoch
        );
    }

    /** Key the scopes and subjects a page marks its rows with. */
    static #marks(page: Page): string {
        return canonicalize([page.scopes ?? null, page.subjects ?? null]);
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
                `replica '${this.name}' of '${this.scope}' has rows in '${undeclared.join(", ")}', but the database no longer declares them`,
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
        left: ReadonlySet<string>,
        options: ApplyOptions,
    ): Promise<LogPosition | undefined> {
        let relayed: LogPosition | undefined;
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
                relayed =
                    (await this.#write(transaction, row.page, delivered, left, options)) ?? relayed;
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
        left: ReadonlySet<string>,
        { open }: ApplyOptions,
    ): Promise<LogPosition | undefined> {
        // batch each table's writes
        const { batches, origin } = await this.#collect(page, delivered, left, open);

        // delete the removed rows children first and write the retained rows parents first
        const ordered = this.tables.flatMap((table): [Table, Batch][] => {
            const batch = batches.get(table);

            return batch === undefined ? [] : [[table, batch]];
        });
        for (const [table, batch] of ordered.toReversed()) {
            const keys = batch.removed.map((row) => Key.name(table, row));
            await this.#exclude(transaction, table, keys);
        }
        for (const [table, batch] of ordered) {
            await this.#writeShown(transaction, table, batch);
        }

        // retain the groups
        await this.#storeResults(transaction, page.results ?? []);

        return origin;
    }

    /** Collect a page's changes into table batches, past those of the tables the run left. */
    async #collect(
        page: Page,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
        left: ReadonlySet<string>,
        open: ApplyOptions["open"],
    ): Promise<{
        readonly batches: Map<Table, Batch>;
        readonly origin: LogPosition | undefined;
    }> {
        // collect each change in commit order
        const batches = new Map<Table, Batch>();
        let origin: LogPosition | undefined;
        for (const change of page.changes) {
            const table = this.#replicated.get(change.table);

            // take a relaying source's replica position as the home position
            if (change.table === replicaPosition[TABLE].sqlName) {
                origin = this.#relayed(change);
            }
            // pass over a change of a table the run left, whose rows it took out
            else if (left.has(change.table)) {
                continue;
            }
            // refuse a table outside the replica
            else if (table === undefined) {
                throw SyncError.of(
                    "STREAM_UNREACHABLE",
                    `could not apply page table '${change.table}' outside replica '${this.name}'`,
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
    #relayed(change: RowChange): LogPosition {
        const origin =
            change.operation === "delete"
                ? undefined
                : ReplicaRecord.origin(
                      Table.selectSchema(replicaPosition).parse(
                          Table.decode(replicaPosition, change.row),
                      ),
                  );
        if (origin === undefined) {
            throw DatabaseError.of(
                "EPOCH_CONFLICT",
                `cannot follow '${this.scope}': the source stopped replicating it`,
            );
        }

        return origin;
    }

    /** Write a table's retained rows of one page and record them as included. */
    async #writeShown(transaction: DatabaseConnection, table: Table, batch: Batch): Promise<void> {
        // write the retained rows with hidden columns cleared and record them as included
        await transaction.upsert(
            table,
            await this.#conceal(transaction, table, batch.shown, batch.hidden),
        );
        await this.#include(transaction, table, batch.shown, batch.hidden);
    }

    /** Retain or let go of groups, or every group of a query, a batch of groups per statement. */
    async #storeResults(
        transaction: DatabaseConnection,
        results: readonly ResultChange[],
    ): Promise<void> {
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
            // refuse a retained group without a row count
            else if (result.rows === undefined) {
                throw SyncError.of(
                    "STREAM_UNREACHABLE",
                    `could not read the rows of a group of '${result.query}' in the page`,
                );
            }
            // write the group's values
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
        // read a table whose rows its scope and condition cannot select here by the rows the replica includes
        if (this.#isKeyed(table)) {
            await this.#pruneIncluded(database, table, delivered);
        }
        // read any other table by the rows of the replica's scope and condition
        else {
            await this.#pruneSelected(database, table, delivered);
        }
    }

    /** Decide whether only the replica's keys select a table's rows here. */
    #isKeyed(table: Table): boolean {
        const where = this.where.get(table);
        const isRelated =
            where !== undefined &&
            Predicate.relations(Condition.resolve(where, Namespace.fields(table, { extras: {} })))
                .length > 0;

        return (
            this.within.has(table) ||
            this.everywhere.has(table) ||
            this.includes.has(table) ||
            isRelated
        );
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
                .select({ key: replicaKey.key })
                .from(replicaKey)
                .where(
                    and(
                        this.#match(replicaKey),
                        eq(replicaKey.table, name),
                        last === undefined ? undefined : gt(replicaKey.key, last),
                    ),
                )
                .orderBy(asc(replicaKey.key))
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

    /** Clear the columns the replica hides on some rows, except a column another replica shows. */
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
            // leave a hidden column as stored while another replica shows it
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
                .select({ key: replicaKey.key, concealed: replicaKey.concealed })
                .from(replicaKey)
                .where(
                    and(
                        not(this.#match(replicaKey)),
                        eq(replicaKey.table, name),
                        inArray(replicaKey.key, keys.slice(start, start + PRUNE_BATCH)),
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
            replicaKey,
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
        // leave the predefined rows to the database, which stores them identical everywhere
        const declared = keys.filter((key) => !Key.isPredefined(table, Key.parse(table, key)));

        // take the rows out of the replica
        const name = table[TABLE].sqlName;
        for (let start = 0; start < declared.length; start += PRUNE_BATCH) {
            const batch = declared.slice(start, start + PRUNE_BATCH);
            const selected = and(
                this.#match(replicaKey),
                eq(replicaKey.table, name),
                inArray(replicaKey.key, batch),
            );
            const shown = await database
                .select({ key: replicaKey.key, concealed: replicaKey.concealed })
                .from(replicaKey)
                .where(selected);
            await database.delete(replicaKey).where(selected);

            // delete the rows no other replica includes
            const others = await this.#others(database, table, batch);
            const deleted = batch
                .filter((key) => !others.has(key))
                .map((key) => Key.parse(table, key));
            await database.remove(table, deleted);

            // clear the columns only this replica showed on the rows the others retain
            await Replica.#clearShown(database, table, shown, others);
        }
    }

    /** Match the replica's rows of a bookkeeping table keyed by name and scope. */
    #match(
        table:
            | typeof replica
            | typeof replicaScope
            | typeof replicaSubscription
            | typeof replicaPosition
            | typeof replicaPage
            | typeof replicaResult
            | typeof replicaKey,
    ): SQL {
        return and(eq(table.name, this.name), eq(table.scope, this.scope));
    }

    /** Read a batch of the keys the replica includes in a table. */
    async #keys(database: DatabaseConnection, table: Table): Promise<string[]> {
        const rows = await database
            .select({ key: replicaKey.key })
            .from(replicaKey)
            .where(and(this.#match(replicaKey), eq(replicaKey.table, table[TABLE].sqlName)))
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
        // leave a predefined row to the database, which stores it identical everywhere
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

        // add the row to the retained or removed ones
        const batch = batches.get(table) ?? { shown: [], hidden: [], removed: [] };
        batches.set(table, batch);
        if (change.operation === "delete") {
            batch.removed.push(row);
        } else {
            batch.shown.push(row);
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

        // remove the let go groups and write the retained ones
        await transaction.remove(
            replicaResult,
            entries.flatMap(({ key, row }) => (row === null ? [key] : [])),
        );
        await transaction.upsert(
            replicaResult,
            entries.flatMap(({ row }) => (row === null ? [] : [row])),
        );
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
            throw new TypeError(`node '${node.name}' predicts groups without an aggregate`);
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

        // rebase onto a snapshot, a staged run or groups
        if (run.isSnapshot || run.staged > 0 || (page.results ?? []).length > 0) {
            return prediction;
        }

        // rebase onto a page changing a layer's tables or including its mutation, none without layers
        const layers = await prediction.layers(transaction);
        const tables = new Set(page.changes.map((change) => change.table));
        const isRebased =
            layers.length > 0 &&
            (layers.some((read) => read.some((table) => tables.has(table))) ||
                (await prediction.includes(transaction, page.origin ?? page.position)));

        return isRebased ? prediction : undefined;
    }

    /** Read the stored rows of a table by the keys of some rows. */
    static #storedRows(
        database: DatabaseConnection,
        table: Table,
        keys: readonly Row[],
    ): Promise<Row[]> {
        const primary = table[TABLE].key;

        return database.snapshot().select(
            table,
            primary,
            keys.map((key) => primary.map((column) => key[column])),
        );
    }

    /** Clear the columns only a replica showed on rows that other replicas retain. */
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

    /** Decide the home origin a run records. */
    static #originColumns(
        page: Page,
        run: Stage,
        record: ReplicaRecord | undefined,
        relayed: LogPosition | undefined,
    ): Pick<ReplicaPosition, "originLog" | "originEpoch" | "originSequence"> {
        // take a relaying source's position
        if (relayed !== undefined) {
            return {
                originLog: relayed.log,
                originEpoch: relayed.epoch,
                originSequence: relayed.sequence,
            };
        }
        // record no origin for rows at home
        else if (run.isSnapshot || record?.originLog === null) {
            return { originLog: null, originEpoch: null, originSequence: null };
        }
        // refuse a page past a snapshot without the record the snapshot wrote
        else if (record === undefined) {
            throw new TypeError("a replica page past its snapshot has no record");
        }
        // take the recorded relayed position
        else {
            return {
                originLog: record.originLog,
                originEpoch: record.originEpoch,
                originSequence: record.originSequence,
            };
        }
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
    /** The rows the replica retains. */
    readonly shown: Row[];
    /** The columns the replica hides on each retained row. */
    readonly hidden: string[][];
    /** The rows the replica lets go. */
    readonly removed: Row[];
}

/** A group's last change in one page. */
interface GroupWrite {
    /** The group's key. */
    readonly key: Key<typeof replicaResult>;
    /** The group's retained row or null for a let go group. */
    readonly row: Insert<typeof replicaResult> | null;
}

/** How a replica registers in a database, with the subscription it follows. */
export type RegisterOptions<Followed extends SubscriptionRecord = SubscriptionRecord> =
    | {
          /** The subscription the replica follows. */
          readonly subscription: Followed;
          /** Whose access the source decides the rows by. */
          readonly audience: PublicationAudience;
      }
    | {
          /** No subscription, for a replica without one. */
          readonly subscription?: undefined;
          /** No audience, for a replica without a subscription. */
          readonly audience?: undefined;
      };

/** How a replica applies its source's pages. */
export type ApplyOptions<Followed extends SubscriptionRecord = SubscriptionRecord> =
    RegisterOptions<Followed> & {
        /** The client's predictions to rebase. */
        readonly prediction?: Prediction;
        /** Open the machine-bound values a fenced source sealed to this replica. */
        readonly open?: (table: Table, row: Row) => Promise<Row>;
    };

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
    /** The staged pages. */
    readonly staged: number;
    /** The aggregate groups it retains. */
    readonly results: number;
}
