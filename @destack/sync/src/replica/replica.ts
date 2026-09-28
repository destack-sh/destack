import {
    and,
    asc,
    count,
    defineTable,
    eq,
    gte,
    integer,
    json,
    inArray,
    Key,
    lt,
    primaryKey,
    text,
    TABLE,
    or,
    type DatabaseConnection,
    type SQL,
    type Table,
    decodeRow,
} from "@destack/db";
import { Condition, Order, Scalar, CHAIN_TERMS } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import { SyncError } from "../error/error.ts";
import { schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { describeLog, type LogPosition } from "@destack/db/log";
import { QueryPage, type MutationOutcome, type ResultChange } from "../query/page.ts";
import type { Query } from "../query/query.ts";
import { Node } from "../query/node.ts";
import type { Row } from "@destack/db";
import type { Outbox } from "../outbox/outbox.ts";
import { EVERYONE } from "../feed/audience.ts";
import { changesThroughLog, Dataflow } from "../dataflow/dataflow.ts";
import { Filter } from "../dataflow/filter.ts";
import { Run } from "../dataflow/run.ts";
import { Trace, type Group, type Upstream } from "../dataflow/upstream.ts";
import { View } from "../dataflow/view.ts";
import { upsert } from "./upsert.ts";

/** The staged pages a run's completion reads at once. */
const STAGED_BATCH = 16;

/** The held keys a snapshot's completion reads at once to find the rows it left out. */
const PRUNE_BATCH = 1000;

/**
 * The copies of remote queries a database holds, the source position each holds up to, and the home position its rows reflect.
 *
 * The table is logged, so a database relaying its copies serves each copy's record with its rows, and followers learn the home position they reflect.
 */
export const replica = defineTable(
    "replica",
    {
        /** The copy's name, identifying it together with its scope. */
        name: text("name").notNull(),
        /** The scope whose rows the copy holds. */
        scope: text("scope").notNull(),
        /** The source log's epoch the copy holds, absent until its first complete snapshot. */
        epoch: text("epoch"),
        /** The source log sequence the copy holds up to within the epoch, absent until its first complete snapshot. */
        sequence: integer("sequence"),
        /** The epoch of the home log the copied rows reflect, absent when the source is their home. */
        originEpoch: text("origin_epoch"),
        /** The home log sequence the copied rows reflect within its epoch, absent when the source is their home. */
        originSequence: integer("origin_sequence"),
        /** The queries the copy holds, in its follower's terms, absent while it follows the whole scope. */
        queries: json("queries", schema.json()),
        /** The shape of the copied tables' logged columns the copy's rows have, absent until its first complete snapshot. */
        shape: text("shape"),
        /** The time the copied rows were last confirmed current with their home, in UTC epoch milliseconds. */
        confirmedAt: integer("confirmed_at").notNull(),
    },
    {
        log: {},
        constraints: (copy) => [
            primaryKey({ name: "replica_key", columns: [copy.name, copy.scope] }),
        ],
    },
);

/** The pages of a copy's run in progress, staged until the page completing the run arrives. */
export const replicaPage = defineTable(
    "replica_page",
    {
        /** The copy's name. */
        name: text("name").notNull(),
        /** The copy's scope. */
        scope: text("scope").notNull(),
        /** The page's place in the run, from zero. */
        index: integer("index").notNull(),
        /** The page as the source sent it. */
        page: json("page", QueryPage).notNull(),
    },
    {
        constraints: (staged) => [
            primaryKey({
                name: "replica_page_key",
                columns: [staged.name, staged.scope, staged.index],
            }),
        ],
    },
);

/** The aggregate groups a copy holds, as its source's aggregate queries measure them. */
export const replicaResult = defineTable(
    "replica_result",
    {
        /** The copy's name. */
        name: text("name").notNull(),
        /** The copy's scope. */
        scope: text("scope").notNull(),
        /** The aggregate query, by its path of names. */
        query: text("query").notNull(),
        /** The group's values in their JSON form, as canonical JSON text naming the group. */
        group: text("group").notNull(),
        /** The group's measures by name. */
        values: json("values", schema.record(schema.string(), Scalar)).notNull(),
        /** The rows the group holds. */
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
        constraints: (result) => [
            primaryKey({
                name: "replica_result_key",
                columns: [result.name, result.scope, result.query, result.group],
            }),
        ],
    },
);

/** The tables a database holding copies includes. */
export const REPLICA_TABLES: readonly Table[] = [replica, replicaPage, replicaResult];

/** A local copy of the rows one scope holds in a remote database's tables, every row or those some conditions select. */
export class Replica {
    /** The copy's name, identifying it together with its scope. */
    readonly name: string;
    /** The scope whose rows the copy holds. */
    readonly scope: string;
    /** The tables the copy writes, parents before the tables referencing them. */
    readonly tables: readonly Table[];
    /** The rows the copy holds of each table within the scope, every row of a table without a condition. */
    readonly where: ReadonlyMap<Table, Condition>;
    /** The copied tables, by SQL name. */
    readonly #copied: ReadonlyMap<string, Table>;
    /** The shape of the copied tables' logged columns, which a copy of another shape snapshots again to hold. */
    readonly #shape: string;

    /** Name a copy of a scope's rows in some tables, each of which must log every column an insert requires. */
    constructor(definition: {
        readonly name: string;
        readonly scope: string;
        readonly tables: readonly Table[];
        readonly where?: ReadonlyMap<Table, Condition>;
    }) {
        // retain the copy's identity and require its tables to carry whole rows in their log
        this.name = definition.name;
        this.scope = definition.scope;
        this.tables = definition.tables;
        this.where = definition.where ?? new Map();
        for (const table of this.tables) {
            requireCopyable(table);
        }
        this.#copied = new Map(this.tables.map((table) => [table[TABLE].sqlName, table]));
        this.#shape = canonicalize(
            this.tables.map((table) => [
                table[TABLE].sqlName,
                Object.values(table[TABLE].logged).map(({ definition }) => [
                    definition.name,
                    definition.kind,
                    definition.nullable,
                ]),
            ]),
        );
    }

    /**
     * The queries the copy holds: the rows of its scope in each table its conditions select, by table name.
     *
     * They include the source's own record of a copy of the same name and scope, which a relaying source holds and a home does not.
     */
    get queries(): Record<string, Query> {
        return Object.fromEntries([
            ...this.tables.map((table) => {
                const where = this.where.get(table);

                return [
                    table[TABLE].sqlName,
                    { table, scopes: [this.scope], ...(where === undefined ? {} : { where }) },
                ];
            }),
            [
                replica[TABLE].sqlName,
                { table: replica, scopes: [this.scope], where: Condition.eq("name", this.name) },
            ],
        ]);
    }

    /** Report whether a database copies a scope instead of holding its source. */
    static async isCopied(database: DatabaseConnection, scope: string): Promise<boolean> {
        const [copy] = await database
            .select({ name: replica.name })
            .from(replica)
            .where(eq(replica.scope, scope))
            .limit(1);

        return copy !== undefined;
    }

    /** Wait until a scope is copied and every copy reflects its home up to a position; false once the signal aborts. */
    static async reach(
        database: DatabaseConnection,
        scope: string,
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<boolean> {
        return database.log.until(async () => {
            // read the home position every copy of the scope reflects
            const records = await database.select().from(replica).where(eq(replica.scope, scope));
            const copies = records.map((record) => recordOrigin(record));

            // refuse a position of a history the home no longer holds
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

    /** Refuse relaying a copy of a scope before it holds a position, whose followers would mistake the relay for the home. */
    static async requireRelayable(
        database: DatabaseConnection,
        name: string,
        scope: string,
    ): Promise<void> {
        const [record] = await database
            .select()
            .from(replica)
            .where(and(eq(replica.name, name), eq(replica.scope, scope)));
        if (record !== undefined && recordOrigin(record) === undefined) {
            throw new SyncError("STALE", `copy of ${scope} holds no position yet`);
        }
    }

    /** Read the home position each complete copy of a name among some scopes reflects, and when its home last confirmed it, by scope. */
    static async origins(
        database: DatabaseConnection,
        name: string,
        scopes: readonly string[],
    ): Promise<Map<string, Origin>> {
        const records = await database
            .select()
            .from(replica)
            .where(and(eq(replica.name, name), inArray(replica.scope, scopes)));

        return new Map(
            records.flatMap((record) => {
                const origin = recordOrigin(record);

                return origin === undefined ? [] : [[record.scope, origin]];
            }),
        );
    }

    /** Read the position the copy holds its source up to, absent before its first complete snapshot and once its tables changed shape. */
    async position(database: DatabaseConnection): Promise<LogPosition | undefined> {
        return this.#held(await this.#record(database));
    }

    /** Read the copy's record, absent before it registered. */
    async #record(database: DatabaseConnection): Promise<typeof replica.$inferSelect | undefined> {
        const [record] = await database
            .select()
            .from(replica)
            .where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));

        return record;
    }

    /** Read the position a record holds its source up to, absent before its first complete snapshot and once its tables changed shape. */
    #held(record: typeof replica.$inferSelect | undefined): LogPosition | undefined {
        return record === undefined ||
            record.epoch === null ||
            record.sequence === null ||
            record.shape !== this.#shape
            ? undefined
            : { epoch: record.epoch, sequence: record.sequence };
    }

    /**
     * Describe the copy for inspection: the position it holds, whether its tables still have the shape it copied, and what it holds and stages.
     *
     * A copy of another shape holds no position, and snapshots again when it next follows.
     */
    async inspect(database: DatabaseConnection): Promise<ReplicaInspection> {
        // read the copy's record, its staged pages and its groups
        const [row] = await database
            .select({
                epoch: replica.epoch,
                sequence: replica.sequence,
                originEpoch: replica.originEpoch,
                originSequence: replica.originSequence,
                queries: replica.queries,
                shape: replica.shape,
                confirmedAt: replica.confirmedAt,
            })
            .from(replica)
            .where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));
        const [staged] = await database
            .select({ pages: count() })
            .from(replicaPage)
            .where(this.#staged());
        const [results] = await database
            .select({ groups: count() })
            .from(replicaResult)
            .where(and(eq(replicaResult.name, this.name), eq(replicaResult.scope, this.scope)));

        return {
            name: this.name,
            scope: this.scope,
            ...(row?.epoch === null || row?.sequence === null || row === undefined
                ? {}
                : { position: { epoch: row.epoch, sequence: row.sequence } }),
            ...(row?.originEpoch === null || row?.originSequence === null || row === undefined
                ? {}
                : { origin: { epoch: row.originEpoch, sequence: row.originSequence } }),
            isShaped: row?.shape === this.#shape,
            queries:
                typeof row?.queries === "object" && row.queries !== null
                    ? Object.keys(row.queries)
                    : [],
            ...(row === undefined ? {} : { confirmedAt: row.confirmedAt }),
            staged: staged!.pages,
            results: results!.groups,
        };
    }

    /** Read the queries the copy holds, in its follower's terms. */
    async holding(database: DatabaseConnection): Promise<unknown> {
        const [row] = await database
            .select({ queries: replica.queries })
            .from(replica)
            .where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));

        return row?.queries ?? undefined;
    }

    /**
     * Read a query's rows from the copy as it holds them, local predictions included, with what each includes.
     *
     * A child type's rows nest as a list; the object a key join names nests as that row or null.
     * An aggregate include nests each row's measures, or a list of its groups when it groups by columns.
     */
    async rows(
        database: DatabaseConnection,
        name: string,
        query: Omit<Query, "scopes">,
        outbox?: Outbox,
    ): Promise<Row[]> {
        // hold the query over the copy as of its latest position, measuring through the source's groups
        const dataflow = new Dataflow(
            { [name]: { ...query, scopes: [this.scope] } },
            {
                audience: EVERYONE,
                database,
                upstream: this.upstream(database, outbox),
                changesThrough: changesThroughLog(database),
                isMaterialized: true,
            },
        );
        await dataflow.fill(await View.latest(database));

        return dataflow.read(name);
    }

    /**
     * Read an aggregate query's groups from the copy, with the local predictions the source has not counted yet.
     *
     * Predictions add to counts and sums, and to extremes they pass; averages wait for the source.
     */
    async results(
        database: DatabaseConnection,
        name: string,
        query: Omit<Query, "scopes">,
        outbox?: Outbox,
    ): Promise<
        { readonly group: Record<string, Scalar>; readonly values: Record<string, Scalar> }[]
    > {
        const node = new Node(name, { ...query, scopes: [this.scope] }, [this.scope]);
        const groups = await this.groupsOf(database, node, outbox);

        return groups.map(({ group, values }) => ({ group, values }));
    }

    /** Measure relations and aggregates as the source measured them for the copy, with the local predictions, for a dataflow over the copy. */
    upstream(database: DatabaseConnection, outbox: Outbox | undefined): Upstream {
        return {
            watches: [{ table: replicaResult, scopes: [this.scope] }],
            groups: (node) => this.groupsOf(database, node, outbox),
            groupOf: (change) => {
                const row = (change.after ?? change.before) as Row | undefined;

                return change.table !== replicaResult ||
                    row === undefined ||
                    row.name !== this.name ||
                    row.scope !== this.scope
                    ? undefined
                    : {
                          query: row.query as string,
                          group: JSON.parse(row.group as string) as Record<string, Scalar>,
                      };
            },
        };
    }

    /** Read a node's aggregate groups holding rows as the source measured them, with the local predictions it has not counted yet. */
    async groupsOf(
        database: DatabaseConnection,
        node: Node,
        outbox: Outbox | undefined,
    ): Promise<Group[]> {
        // read the groups the source measured
        const rows = await database
            .select({
                group: replicaResult.group,
                values: replicaResult.values,
                rows: replicaResult.rows,
                parts: replicaResult.parts,
            })
            .from(replicaResult)
            .where(
                and(
                    eq(replicaResult.name, this.name),
                    eq(replicaResult.scope, this.scope),
                    eq(replicaResult.query, node.name),
                ),
            );
        const groups = new Map<string, Predicted>(
            rows.map((row) => [
                row.group,
                {
                    group: JSON.parse(row.group) as Record<string, Scalar>,
                    values: { ...row.values },
                    rows: row.rows,
                    parts: { ...row.parts },
                },
            ]),
        );

        // decide the predicted images of the node's table as the source would, following relations through the copy
        const table = node.table[TABLE].sqlName;
        const predicted = (outbox === undefined ? [] : await outbox.predicted(database)).filter(
            (change) => change.table === table,
        );
        const images = predicted.flatMap((change) => {
            // take the prediction's image before and after
            const row = decodeRow(node.table, change.row);
            const before =
                change.operation === "insert"
                    ? undefined
                    : change.operation === "update"
                      ? decodeRow(node.table, change.before!)
                      : row;
            const after = change.operation === "delete" ? undefined : row;

            return [
                ...(before === undefined ? [] : [{ image: before, sign: -1 as const }]),
                ...(after === undefined ? [] : [{ image: after, sign: 1 as const }]),
            ];
        });
        if (images.length > 0) {
            const run = new Run(await View.latest(database), EVERYONE, "collect");
            const filter = new Filter(node, new Trace(this.upstream(database, outbox)));
            await filter.prepare(
                images.map(({ image }) => image),
                run,
            );

            // add the candidates to their groups' counts, sums, averages and extremes
            for (const { image, sign } of images) {
                if (filter.isCandidate(image, run)) {
                    predict(node, groups, filter.resolve(image, run), sign);
                }
            }
        }

        return [...groups.values()].filter((group) => group.rows > 0);
    }

    /** Record the queries the copy holds once a run holding them completed. */
    async hold(database: DatabaseConnection, queries: unknown): Promise<void> {
        await database
            .update(replica)
            .set({ queries: schema.json().parse(queries) })
            .where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));
    }

    /** Record the copy before its first snapshot, so the database knows it copies the scope. */
    async register(database: DatabaseConnection): Promise<void> {
        await database
            .insert(replica)
            .values({
                name: this.name,
                scope: this.scope,
                epoch: null,
                sequence: null,
                queries: null,
                confirmedAt: Date.now(),
            })
            .onConflictDoNothing();
    }

    /** Apply one stream of pages, staging each run's pages and applying the run at once when it completes. */
    async *apply(
        database: DatabaseConnection,
        pages: AsyncIterable<QueryPage> | Iterable<QueryPage>,
        outbox?: Outbox,
    ): AsyncGenerator<QueryPage> {
        // drop what an earlier stream staged, since every stream restarts from the recorded position
        await database.delete(replicaPage).where(this.#staged());
        let staged = 0;
        let isSnapshot = false;
        for await (const page of pages) {
            // drop the staged pages a snapshot replaces, and note whether a run starts with one
            if (page.reset && staged > 0) {
                await database.delete(replicaPage).where(this.#staged());
                staged = 0;
            }
            if (staged === 0) {
                isSnapshot = page.reset;
            }

            // apply the run a page completes
            if (page.complete) {
                await this.#complete(database, page, { staged, isSnapshot }, outbox);
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

    /** Apply the pages a source streams from the recorded position until the signal aborts. */
    async follow(
        database: DatabaseConnection,
        source: (after: LogPosition | undefined, signal: AbortSignal) => AsyncIterable<QueryPage>,
        signal: AbortSignal,
        outbox?: Outbox,
    ): Promise<void> {
        await this.register(database);
        const pages = source(await this.position(database), signal);
        for await (const _page of this.apply(database, pages, outbox)) {
            // apply each page as it arrives
        }
    }

    /** Apply a run's staged pages and the page completing it in one transaction, rebasing local predictions onto them. */
    async #complete(
        database: DatabaseConnection,
        page: QueryPage,
        run: { readonly staged: number; readonly isSnapshot: boolean },
        outbox: Outbox | undefined,
    ): Promise<void> {
        const { staged, isSnapshot } = run;
        await database.transaction(
            async (transaction) => {
                // require changes of the epoch the copy holds, which only a snapshot changes
                const record = await this.#record(transaction);
                const held = this.#held(record);
                if (!isSnapshot && held !== undefined && held.epoch !== page.position.epoch) {
                    throw new DatabaseError(
                        "STALE_EPOCH",
                        `page of epoch ${page.position.epoch} continues a copy of ${held.epoch}`,
                    );
                }

                // write the source's rows as it derived them, reverting local predictions first
                const outcomes: MutationOutcome[] = [];
                await transaction.log.copying(async () => {
                    // revert local predictions before writing the source's rows
                    await outbox?.revert(transaction);

                    // let go of every aggregate group a snapshot replaces
                    if (isSnapshot) {
                        await transaction
                            .delete(replicaResult)
                            .where(
                                and(
                                    eq(replicaResult.name, this.name),
                                    eq(replicaResult.scope, this.scope),
                                ),
                            );
                    }

                    // write the staged pages a batch at a time, then the completing page, noting a snapshot's keys
                    const delivered = isSnapshot
                        ? new Map(
                              this.tables.map((table) => [table[TABLE].sqlName, new Set<string>()]),
                          )
                        : undefined;
                    let relayed: Origin | undefined;
                    for (let start = 0; start < staged; start += STAGED_BATCH) {
                        const batch = await transaction
                            .select({ page: replicaPage.page })
                            .from(replicaPage)
                            .where(
                                and(
                                    this.#staged(),
                                    gte(replicaPage.index, start),
                                    lt(replicaPage.index, start + STAGED_BATCH),
                                ),
                            )
                            .orderBy(asc(replicaPage.index));
                        for (const row of batch) {
                            const written = await this.#write(transaction, row.page, delivered);
                            outcomes.push(...written.outcomes);
                            relayed = written.origin ?? relayed;
                        }
                    }
                    const written = await this.#write(transaction, page, delivered);
                    outcomes.push(...written.outcomes);
                    relayed = written.origin ?? relayed;

                    // remove the rows a snapshot no longer holds, children before parents
                    if (delivered !== undefined) {
                        for (const table of [...this.tables].reverse()) {
                            const name = table[TABLE].sqlName;
                            await this.#prune(transaction, table, delivered.get(name)!);
                        }
                    }

                    // take the home position the rows reflect: a relaying source's record, or the source's own position for a copy of its home
                    const isHome =
                        relayed === undefined && (isSnapshot || record?.originEpoch === null);
                    const origin =
                        relayed !== undefined
                            ? {
                                  originEpoch: relayed.position.epoch,
                                  originSequence: relayed.position.sequence,
                                  confirmedAt: relayed.confirmedAt,
                              }
                            : isHome
                              ? { originEpoch: null, originSequence: null, confirmedAt: Date.now() }
                              : {};

                    // record the position the copy now holds, and drop the staged pages
                    const advanced = {
                        ...page.position,
                        ...origin,
                        shape: this.#shape,
                    };
                    await transaction
                        .insert(replica)
                        .values({
                            name: this.name,
                            scope: this.scope,
                            confirmedAt: record?.confirmedAt ?? Date.now(),
                            ...advanced,
                        })
                        .onConflictDoUpdate({
                            target: [replica.name, replica.scope],
                            set: advanced,
                        });
                    await transaction.delete(replicaPage).where(this.#staged());
                });

                // predict what the source lacks again, settling what a snapshot holds
                await outbox?.replay(transaction, outcomes, isSnapshot ? page.position : undefined);
            },
            { constraints: "deferred" },
        );
    }

    /**
     * Write a page's changes in the order the source committed them, noting a snapshot's keys.
     *
     * Return its outcomes, and the home position a relaying source's record of its own copy names.
     */
    async #write(
        transaction: DatabaseConnection,
        page: QueryPage,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
    ): Promise<{ readonly outcomes: readonly MutationOutcome[]; readonly origin?: Origin }> {
        // write each table's held rows and removals in batches, since a page decides each row once
        const batches = new Map<Table, { held: Row[]; removed: Row[] }>();
        let origin: Origin | undefined;
        for (const change of page.changes) {
            // take the relaying source's record of its copy as the home position the rows reflect
            if (change.table === replica[TABLE].sqlName) {
                origin =
                    change.operation === "delete"
                        ? undefined
                        : recordOrigin(
                              decodeRow(replica, change.row) as typeof replica.$inferSelect,
                          );
                if (origin === undefined) {
                    throw new DatabaseError("STALE_EPOCH", `source stopped copying ${this.scope}`);
                }
                continue;
            }

            // stage a copied row's change
            const table = this.#copied.get(change.table);
            if (!table) {
                throw new TypeError(`page names a table outside the replica: ${change.table}`);
            }
            const row = decodeRow(table, change.row);
            delivered?.get(change.table)!.add(Key.name(table, row));
            const batch = batches.get(table) ?? { held: [], removed: [] };
            batches.set(table, batch);
            if (change.operation === "delete") {
                batch.removed.push(row);
            } else {
                batch.held.push({
                    ...row,
                    ...Object.fromEntries((change.concealed ?? []).map((name) => [name, null])),
                });
            }
        }
        for (const [table, batch] of batches) {
            await remove(transaction, table, batch.removed);
            await upsert(transaction, table, batch.held);
        }

        // hold the aggregate groups
        for (const result of page.results ?? []) {
            await this.#hold(transaction, result);
        }

        return { outcomes: page.outcomes ?? [], ...(origin === undefined ? {} : { origin }) };
    }

    /** Hold an aggregate group's new values, or let go of it, or of every group of its query. */
    async #hold(transaction: DatabaseConnection, result: ResultChange): Promise<void> {
        // match the copy's groups of the query, or one of them
        const matched = and(
            eq(replicaResult.name, this.name),
            eq(replicaResult.scope, this.scope),
            eq(replicaResult.query, result.query),
            result.group === null ? undefined : eq(replicaResult.group, canonicalize(result.group)),
        );

        // let go of a group that holds no rows, or of every group
        if (result.values === null) {
            await transaction.delete(replicaResult).where(matched);
        }
        // refuse a held group that does not count its rows
        else if (result.rows === undefined) {
            throw new TypeError(`page holds a group of ${result.query} without its rows`);
        }
        // hold a group's values
        else {
            const values = { ...result.values };
            const rows = result.rows;
            const parts = result.parts === undefined ? null : { ...result.parts };
            await transaction
                .insert(replicaResult)
                .values({
                    name: this.name,
                    scope: this.scope,
                    query: result.query,
                    group: canonicalize(result.group),
                    values,
                    rows,
                    parts,
                })
                .onConflictDoUpdate({
                    target: [
                        replicaResult.name,
                        replicaResult.scope,
                        replicaResult.query,
                        replicaResult.group,
                    ],
                    set: { values, rows, parts },
                });
        }
    }

    /** Match the pages the copy staged. */
    #staged(): SQL {
        return and(eq(replicaPage.name, this.name), eq(replicaPage.scope, this.scope))!;
    }

    /** Delete a table's rows of the copy that a completed snapshot did not deliver, in key order a batch at a time. */
    async #prune(database: DatabaseConnection, table: Table, delivered: ReadonlySet<string>) {
        // read the held keys in key order, a batch at a time
        const columns = table[TABLE].columns;
        const key = table[TABLE].key;
        const order = Order.complete([], table);
        const fields = Object.fromEntries(key.map((name) => [name, columns[name]!]));
        let last: Record<string, unknown> | undefined;
        do {
            const held = (await database
                .select(fields)
                .from(table)
                .where(
                    and(
                        Condition.render(
                            Condition.all(
                                Node.scoped([this.scope]),
                                this.where.get(table) ?? Condition.all(),
                            ),
                            Condition.bind(table),
                        ),
                        last && Order.after(order, table, last),
                    ),
                )
                .orderBy(...Order.render(order, table))
                .limit(PRUNE_BATCH)) as Record<string, unknown>[];
            last = held.at(-1);

            // delete the batch's stale rows
            await remove(
                database,
                table,
                held.filter((row) => !delivered.has(Key.name(table, row))),
            );
        } while (last !== undefined);
    }
}

/** Read the home position a copy's record reflects, absent before its first complete snapshot. */
function recordOrigin(record: typeof replica.$inferSelect): Origin | undefined {
    // skip a copy that holds no position yet
    if (record.epoch === null || record.sequence === null) {
        return undefined;
    }

    // take a relayed copy's origin, and the source position of a copy of its home
    const position =
        record.originEpoch === null || record.originSequence === null
            ? { epoch: record.epoch, sequence: record.sequence }
            : { epoch: record.originEpoch, sequence: record.originSequence };

    return { position, confirmedAt: record.confirmedAt };
}

/** A group as a prediction changes it: its measures, its rows, and the parts of its averages. */
type Predicted = Group & { rows: number };

/** Add one predicted row image holding its computed values to its group's counts, sums, averages and extremes, or take it away. */
function predict(node: Node, groups: Map<string, Predicted>, row: Row, sign: 1 | -1): void {
    // find or start the row's group, over its computed values too
    const group = node.groupOf(row);
    const key = canonicalize(group);
    const known = groups.get(key) ?? { group, values: node.emptyValues(), rows: 0, parts: {} };
    groups.set(key, known);
    known.rows += sign;

    // add to counts, sums and averages, and pass extremes on additions
    for (const [name, measure] of Object.entries(node.aggregate!.values)) {
        const current = known.values[name] ?? null;
        const value = measure.column === undefined ? null : (row[measure.column] ?? null);
        const json = value === null ? null : (node.json(measure.column!, value) as Scalar);
        if (measure.function === "count") {
            known.values[name] = Number(current ?? 0) + sign;
        } else if (measure.function === "sum" && json !== null) {
            known.values[name] = add(current, json, sign);
        } else if (measure.function === "avg" && json !== null) {
            const part = known.parts[name] ?? { sum: 0, count: 0 };
            const next = { sum: add(part.sum, json, sign), count: part.count + sign };
            known.parts[name] = next;
            known.values[name] = next.count === 0 ? null : Number(next.sum) / next.count;
        } else if (
            (measure.function === "min" || measure.function === "max") &&
            json !== null &&
            sign === 1
        ) {
            const order =
                current === null
                    ? undefined
                    : Order.values(
                          node.fromJson(measure.column!, json),
                          node.fromJson(measure.column!, current),
                      );
            if (order === undefined || (measure.function === "min" ? order < 0 : order > 0)) {
                known.values[name] = json;
            }
        }
    }
}

/** Add a JSON value to a sum, or take it away, exactly for integers beyond the safe range held as text. */
function add(sum: Scalar, value: Scalar, sign: 1 | -1): Scalar {
    return typeof sum === "string" || typeof value === "string"
        ? String(BigInt(sum ?? 0) + BigInt(sign) * BigInt(value ?? 0))
        : Number(sum ?? 0) + sign * Number(value);
}

/** Remove rows from a copied table by key, a batch of keys per statement. */
async function remove(
    database: DatabaseConnection,
    table: Table,
    rows: readonly Row[],
): Promise<void> {
    const size = CHAIN_TERMS;
    for (let start = 0; start < rows.length; start += size) {
        const matches = rows.slice(start, start + size).map((row) => Key.match(table, row));
        await database.delete(table).where(or(...matches)!);
    }
}

/** Require a table to be copyable: its log carries every column an insert requires. */
function requireCopyable(table: Table): void {
    const logged = new Set(describeLog(table)?.columns ?? []);
    for (const column of Object.values(table[TABLE].columns)) {
        const definition = column.definition;
        const isRequired =
            !definition.nullable &&
            definition.default === undefined &&
            definition.runtimeDefault === undefined &&
            definition.generated === undefined;
        if (isRequired && !logged.has(definition.name)) {
            throw new TypeError(
                `replica table ${table[TABLE].name} requires unlogged column ${definition.name}`,
            );
        }
    }
}

/** The home position a copy's rows reflect, and when their home last confirmed them. */
export interface Origin {
    /** The home log position the rows reflect. */
    readonly position: LogPosition;
    /** When the home last confirmed the copied rows current, in UTC epoch milliseconds. */
    readonly confirmedAt: number;
}

/** What a copy holds, as inspection reads it. */
export interface ReplicaInspection {
    /** The copy's name. */
    readonly name: string;
    /** The scope it copies. */
    readonly scope: string;
    /** The source position it recorded, absent before its first complete snapshot. */
    readonly position?: LogPosition;
    /** The home position its rows reflect, absent when the source is their home. */
    readonly origin?: LogPosition;
    /** Whether its tables have the shape it copied, without which it snapshots again. */
    readonly isShaped: boolean;
    /** The names of the queries it holds. */
    readonly queries: readonly string[];
    /** The time its rows were last confirmed current with their home, in UTC epoch milliseconds, absent before it registered. */
    readonly confirmedAt?: number;
    /** The pages it staged of a run in progress. */
    readonly staged: number;
    /** The aggregate groups it holds. */
    readonly results: number;
}
