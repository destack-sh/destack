import {
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
    lt,
    not,
    primaryKey,
    sql,
    text,
    TABLE,
    type DatabaseConnection,
    type SQL,
    type Table,
    decodeRow,
} from "@destack/db";
import { Condition, Order, Scalar } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import { SyncError } from "../error/error.ts";
import { schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { Log, Snapshot, type LogPosition } from "@destack/db/log";
import { QueryPage, type MutationOutcome, type ResultChange } from "../query/page.ts";
import type { Include, Query } from "../query/query.ts";
import { Node } from "../query/node.ts";
import type { Row } from "@destack/db";
import type { Outbox } from "../outbox/outbox.ts";
import { EVERYONE } from "../feed/audience.ts";
import { changesThroughLog, Dataflow } from "../dataflow/dataflow.ts";
import { Filter } from "../dataflow/filter.ts";
import { Run } from "../dataflow/run.ts";
import { Trace, type Group, type Upstream } from "../dataflow/upstream.ts";
import { View } from "../dataflow/view.ts";

/** The staged pages a run's completion reads at once. */
const STAGED_BATCH = 16;

/** The held keys a snapshot's completion reads at once. */
const PRUNE_BATCH = 1000;

/** The copies a database holds, with their source and home positions. */
export const replica = defineTable(
    "replica",
    {
        /** The copy's name. */
        name: text("name").notNull(),
        /** The scope whose rows the copy holds. */
        scope: text("scope").notNull(),
        /** The source log's epoch, absent before the first complete snapshot. */
        epoch: text("epoch"),
        /** The source log sequence, absent before the first complete snapshot. */
        sequence: integer("sequence"),
        /** The home log's epoch the rows reflect, absent when the source is their home. */
        originEpoch: text("origin_epoch"),
        /** The home log sequence the rows reflect, absent when the source is their home. */
        originSequence: integer("origin_sequence"),
        /** The held queries in the follower's terms, absent for the whole scope. */
        queries: json("queries", schema.json()),
        /** The scopes the copy reads, nearest first, absent until its source sends them. */
        scopes: json("scopes", schema.array(schema.string())),
        /** The shape of the copied tables' logged columns. */
        shape: text("shape"),
        /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
        confirmedAt: integer("confirmed_at").notNull(),
    },
    {
        log: {},
        constraints: (copy) => [
            primaryKey({ name: "replica_key", columns: [copy.name, copy.scope] }),
        ],
    },
);

/** The staged pages of a copy's run in progress. */
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

/** The aggregate groups a copy holds. */
export const replicaResult = defineTable(
    "replica_result",
    {
        /** The copy's name. */
        name: text("name").notNull(),
        /** The copy's scope. */
        scope: text("scope").notNull(),
        /** The aggregate query, by its path of names. */
        query: text("query").notNull(),
        /** The group's values as canonical JSON text. */
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

/**
 * The rows of each copy's shape, as the follower stores them: the rows it includes and the columns it hides on each.
 *
 * A row several copies include stays until none includes it, and keeps a column while one of them shows it.
 */
export const replicaRow = defineTable(
    "replica_row",
    {
        /** The copy's name. */
        name: text("name").notNull(),
        /** The copy's scope. */
        scope: text("scope").notNull(),
        /** The row's table, by SQL name. */
        table: text("table").notNull(),
        /** The row's key. */
        key: text("key").notNull(),
        /** The columns the copy's shape hides on the row, by property. */
        concealed: json("concealed", schema.array(schema.string())).notNull(),
    },
    {
        constraints: (row) => [
            primaryKey({
                name: "replica_row_key",
                columns: [row.name, row.scope, row.table, row.key],
            }),
            index("replica_row_table").on(row.table, row.key),
        ],
    },
);

/** The tables of a database holding copies. */
export const replicaTables: readonly Table[] = [replica, replicaPage, replicaResult, replicaRow];

/** A local copy of one scope's rows in a remote database's tables. */
export class Replica {
    /** The copy's name. */
    readonly name: string;
    /** The scope whose rows the copy holds. */
    readonly scope: string;
    /** The copied tables, parents first. */
    readonly tables: readonly Table[];
    /** The condition on each table's copied rows. */
    readonly where: ReadonlyMap<Table, Condition>;
    /** The scope of each table whose rows live outside the copy's scope. */
    readonly scopes: ReadonlyMap<Table, string>;
    /** The tables whose rows are copied from whichever scope they live in. */
    readonly everywhere: ReadonlySet<Table>;
    /** The copied tables, keyed by `id`, whose rows are the scopes of each table's rows, for the tables copied across scopes. */
    readonly within: ReadonlyMap<Table, readonly Table[]>;
    /** The copied tables, by SQL name. */
    readonly #copied: ReadonlyMap<string, Table>;
    /** The shape of the copied tables' logged columns. */
    readonly #shape: Promise<string>;

    /** Define a copy of a scope's rows in some tables. */
    constructor(definition: {
        readonly name: string;
        readonly scope: string;
        readonly tables: readonly Table[];
        readonly where?: ReadonlyMap<Table, Condition>;
        readonly scopes?: ReadonlyMap<Table, string>;
        readonly everywhere?: ReadonlySet<Table>;
        readonly within?: ReadonlyMap<Table, readonly Table[]>;
    }) {
        // keep the identity
        this.name = definition.name;
        this.scope = definition.scope;
        this.tables = definition.tables;
        this.where = definition.where ?? new Map();
        this.scopes = definition.scopes ?? new Map();
        this.everywhere = definition.everywhere ?? new Set();
        this.within = definition.within ?? new Map();
        this.#copied = new Map(this.tables.map((table) => [table[TABLE].sqlName, table]));
        this.#shape = Log.shape(this.tables);
    }

    /** The queries the copy holds, by table name, including the source's own copy record. */
    get queries(): Record<string, Query> {
        // include each table copied across scopes under each table whose rows are its scopes
        const include = (parent: Table): { include?: Record<string, Include> } => {
            const children = this.tables.filter((table) =>
                this.within.get(table)?.includes(parent),
            );

            return children.length === 0
                ? {}
                : {
                      include: Object.fromEntries(
                          children.map((table) => {
                              const where = this.where.get(table);

                              return [
                                  table[TABLE].sqlName,
                                  {
                                      table,
                                      on: { kind: "key", column: "scope", parent: "id" },
                                      ...(where === undefined ? {} : { where }),
                                      ...include(table),
                                  },
                              ];
                          }),
                      ),
                  };
        };

        return Object.fromEntries([
            ...this.tables
                .filter((table) => !this.within.has(table))
                .map((table) => {
                    const where = this.where.get(table);
                    const scope = this.scopes.get(table) ?? this.scope;

                    return [
                        table[TABLE].sqlName,
                        {
                            table,
                            scopes: this.everywhere.has(table) ? "every" : [scope],
                            ...(where === undefined ? {} : { where }),
                            ...include(table),
                        },
                    ];
                }),
            [
                replica[TABLE].sqlName,
                { table: replica, scopes: [this.scope], where: Condition.eq("name", this.name) },
            ],
        ]);
    }

    /** Match the rows of a table keyed by a text `id` that a named copy includes, as SQL. */
    static includes(name: string, table: Table): SQL {
        const { sqlName, columns } = table[TABLE];
        const key = sql`'["' || ${sqlName} || '","' || ${columns.id} || '"]'`;

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
            // read each copy's home position
            const records = await database.select().from(replica).where(eq(replica.scope, scope));
            const copies = records.map((record) => recordOrigin(record));

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

    /** Refuse relaying a copy that holds no position yet. */
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

    /** Read each complete copy's home position and confirmation time, by scope. */
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

    /** Read the source position the copy holds, absent before its first snapshot or after a shape change. */
    async position(database: DatabaseConnection): Promise<LogPosition | undefined> {
        return this.#held(await this.#record(database), await this.#shape);
    }

    /** Read the copy's record, absent before it registered. */
    async #record(database: DatabaseConnection): Promise<typeof replica.$inferSelect | undefined> {
        const [record] = await database
            .select()
            .from(replica)
            .where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));

        return record;
    }

    /** Read a record's source position, absent before its first snapshot or after a shape change. */
    #held(record: typeof replica.$inferSelect | undefined, shape: string): LogPosition | undefined {
        return record === undefined ||
            record.epoch === null ||
            record.sequence === null ||
            record.shape !== shape
            ? undefined
            : { epoch: record.epoch, sequence: record.sequence };
    }

    /** Describe the copy's position, shape, holdings and staged pages. */
    async inspect(database: DatabaseConnection): Promise<ReplicaInspection> {
        // read the record, staged pages and groups
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
            isShaped: row?.shape === (await this.#shape),
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

    /** Read the scope chain the copy reads, nearest first: its own scope alone until its source sends it. */
    async chain(database: DatabaseConnection): Promise<readonly string[]> {
        const [row] = await database
            .select({ scopes: replica.scopes })
            .from(replica)
            .where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));

        return row?.scopes ?? [this.scope];
    }

    /** Read a query's rows from the copy, predictions included. */
    async rows(
        database: DatabaseConnection,
        name: string,
        query: Omit<Query, "scopes">,
        outbox?: Outbox,
    ): Promise<Row[]> {
        // hold the query over the copy at its latest position
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

    /** Read an aggregate query's groups from the copy, predictions included. */
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

    /** Measure relations and aggregates through the source's groups, predictions included. */
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

    /** Read a node's source groups with the uncounted predictions. */
    async groupsOf(
        database: DatabaseConnection,
        node: Node,
        outbox: Outbox | undefined,
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

        // decide the predicted images of the node's table
        const table = node.table[TABLE].sqlName;
        const predicted = (outbox === undefined ? [] : await outbox.predicted(database)).filter(
            (change) => change.table === table,
        );
        const images = predicted.flatMap((change) => {
            // take the prediction's images
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

            // add the candidates to their groups
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

    /** Keep the copied rows as the database's own. */
    async promote(database: DatabaseConnection): Promise<void> {
        await database.transaction(async (transaction) => {
            // require a complete copy
            if ((await this.position(transaction)) === undefined) {
                throw new SyncError("STALE", `copy of ${this.scope} holds no position yet`);
            }

            // drop the copy's record, staged pages and groups
            const own = (
                table:
                    | typeof replica
                    | typeof replicaPage
                    | typeof replicaResult
                    | typeof replicaRow,
            ) => and(eq(table.name, this.name), eq(table.scope, this.scope));
            await transaction.delete(replicaPage).where(own(replicaPage));
            await transaction.delete(replicaResult).where(own(replicaResult));
            await transaction.delete(replicaRow).where(own(replicaRow));
            await transaction.delete(replica).where(own(replica));
        });
    }

    /** Record the copy before its first snapshot. */
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

    /** Apply one stream of pages, staging each run until it completes. */
    async *apply(
        database: DatabaseConnection,
        pages: AsyncIterable<QueryPage> | Iterable<QueryPage>,
        outbox?: Outbox,
        request?: unknown,
    ): AsyncGenerator<QueryPage> {
        // drop an earlier stream's staged pages
        await database.delete(replicaPage).where(this.#staged());
        let staged = 0;
        let isSnapshot = false;
        for await (const page of pages) {
            // drop staged pages a snapshot replaces
            if (page.reset && staged > 0) {
                await database.delete(replicaPage).where(this.#staged());
                staged = 0;
            }
            if (staged === 0) {
                isSnapshot = page.reset;
            }

            // apply the run a page completes
            if (page.complete) {
                await this.#complete(database, page, { staged, isSnapshot }, outbox, request);
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

    /** Apply a source's pages until the signal aborts, resuming each stream that ends, from scratch after a changed request. */
    async follow(
        database: DatabaseConnection,
        source: (after: LogPosition | undefined, signal: AbortSignal) => AsyncIterable<QueryPage>,
        signal: AbortSignal,
        options: {
            /** The client's predictions to rebase. */
            readonly outbox?: Outbox;
            /** What the copy asks its source for, recorded as it completes a run. */
            readonly request?: unknown;
        } = {},
    ): Promise<void> {
        // apply each stream from the recorded position until aborted
        await this.register(database);
        while (!signal.aborted) {
            // resume only the request the copy completed
            const isResumed =
                options.request === undefined ||
                canonicalize((await this.holding(database)) ?? null) ===
                    canonicalize(options.request);
            const after = isResumed ? await this.position(database) : undefined;
            let isReceived = false;
            const pages = source(after, signal);
            for await (const _page of this.apply(
                database,
                pages,
                options.outbox,
                options.request,
            )) {
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
        page: QueryPage,
        run: { readonly staged: number; readonly isSnapshot: boolean },
        outbox: Outbox | undefined,
        request: unknown,
    ): Promise<void> {
        // apply and rebase in one transaction
        const { staged, isSnapshot } = run;
        await database.transaction(
            async (transaction) => {
                // rebase onto pages that reach a prediction
                const isRebased =
                    outbox !== undefined &&
                    (isSnapshot ||
                        staged > 0 ||
                        (page.results ?? []).length > 0 ||
                        (page.outcomes ?? []).length > 0 ||
                        (page.changes.length > 0 &&
                            (await outbox.reaches(
                                transaction,
                                new Set(page.changes.map((change) => change.table)),
                            ))));
                const rebased = isRebased ? outbox : undefined;

                // require the held epoch outside a snapshot
                const record = await this.#record(transaction);
                const held = this.#held(record, await this.#shape);
                if (!isSnapshot && held !== undefined && held.epoch !== page.position.epoch) {
                    throw new DatabaseError(
                        "STALE_EPOCH",
                        `page of epoch ${page.position.epoch} continues a copy of ${held.epoch}`,
                    );
                }

                // write the source's rows after reverting predictions
                const outcomes: MutationOutcome[] = [];
                await transaction.log.copying(async () => {
                    // revert local predictions
                    await rebased?.revert(transaction);

                    // let go of every group on a snapshot
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

                    // write the staged and completing pages
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

                    // remove the rows a snapshot left out, children first
                    if (delivered !== undefined) {
                        for (const table of [...this.tables].reverse()) {
                            const name = table[TABLE].sqlName;
                            await this.#prune(transaction, table, delivered.get(name)!);
                        }
                    }

                    // take the home position the rows reflect
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

                    // record the new position and scopes, and drop the staged pages
                    const advanced = {
                        ...page.position,
                        ...origin,
                        ...(page.scopes === undefined ? {} : { scopes: page.scopes }),
                        ...(request === undefined ? {} : { queries: schema.json().parse(request) }),
                        shape: await this.#shape,
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

                // predict again what the source lacks
                await rebased?.replay(
                    transaction,
                    outcomes,
                    isSnapshot ? page.position : undefined,
                );
            },
            { constraints: "deferred" },
        );
    }

    /** Write a page's changes in commit order, returning its outcomes and home position. */
    async #write(
        transaction: DatabaseConnection,
        page: QueryPage,
        delivered: ReadonlyMap<string, Set<string>> | undefined,
    ): Promise<{ readonly outcomes: readonly MutationOutcome[]; readonly origin?: Origin }> {
        // batch each table's writes
        const batches = new Map<Table, { held: Row[]; hidden: string[][]; removed: Row[] }>();
        let origin: Origin | undefined;
        for (const change of page.changes) {
            // take a relaying source's copy record as the home position
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
            const batch = batches.get(table) ?? { held: [], hidden: [], removed: [] };
            batches.set(table, batch);
            if (change.operation === "delete") {
                batch.removed.push(row);
            } else {
                batch.held.push(row);
                batch.hidden.push(change.concealed ?? []);
            }
        }
        for (const [table, batch] of batches) {
            await this.#exclude(
                transaction,
                table,
                batch.removed.map((row) => Key.name(table, row)),
            );
            await transaction.upsert(
                table,
                await this.#conceal(transaction, table, batch.held, batch.hidden),
            );
            await this.#include(transaction, table, batch.held, batch.hidden);
        }

        // hold the groups
        for (const result of page.results ?? []) {
            await this.#hold(transaction, result);
        }

        return { outcomes: page.outcomes ?? [], ...(origin === undefined ? {} : { origin }) };
    }

    /** Hold or let go of a group, or of every group of its query. */
    async #hold(transaction: DatabaseConnection, result: ResultChange): Promise<void> {
        // match the query's groups, or one of them
        const matched = and(
            eq(replicaResult.name, this.name),
            eq(replicaResult.scope, this.scope),
            eq(replicaResult.query, result.query),
            result.group === null ? undefined : eq(replicaResult.group, canonicalize(result.group)),
        );

        // let go of an empty group or every group
        if (result.values === null) {
            await transaction.delete(replicaResult).where(matched);
        }
        // refuse a held group without a row count
        else if (result.rows === undefined) {
            throw new TypeError(`page holds a group of ${result.query} without its rows`);
        }
        // hold the group's values
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

    /** Take the rows a completed snapshot left out of the copy, a batch at a time in key order. */
    async #prune(database: DatabaseConnection, table: Table, delivered: ReadonlySet<string>) {
        // read a table copied across scopes by the rows the copy includes
        if (this.within.has(table) || this.everywhere.has(table)) {
            const name = table[TABLE].sqlName;
            let last: string | undefined;
            do {
                const rows = await database
                    .select({ key: replicaRow.key })
                    .from(replicaRow)
                    .where(
                        and(
                            this.#included(),
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
        // read any other table by the rows of the copy's scope and condition
        else {
            const columns = table[TABLE].columns;
            const key = table[TABLE].key;
            const order = Order.complete([], table);
            const fields = Object.fromEntries(key.map((name) => [name, columns[name]!]));
            let last: Record<string, unknown> | undefined;
            do {
                const rows = (await database
                    .select(fields)
                    .from(table)
                    .where(
                        and(
                            Condition.render(
                                Condition.all(
                                    Node.scoped([this.scopes.get(table) ?? this.scope]),
                                    this.where.get(table) ?? Condition.all(),
                                ),
                                Condition.bind(table),
                            ),
                            last && Order.after(order, table, last),
                        ),
                    )
                    .orderBy(...Order.render(order, table))
                    .limit(PRUNE_BATCH)) as Record<string, unknown>[];
                last = rows.at(-1);

                // take out the rows the snapshot left out
                const stale = rows
                    .map((row) => Key.name(table, row))
                    .filter((name) => !delivered.has(name));
                await this.#exclude(database, table, stale);
            } while (last !== undefined);
        }
    }

    /** Clear the columns the copy's shape hides on some rows, keeping a column another copy's shape shows. */
    async #conceal(
        database: DatabaseConnection,
        table: Table,
        rows: readonly Row[],
        hidden: readonly (readonly string[])[],
    ): Promise<Row[]> {
        // read what the other copies' shapes hide on the rows hiding columns
        const hiding = rows.filter((_, position) => hidden[position]!.length > 0);
        const others = await this.#others(
            database,
            table,
            hiding.map((row) => Key.name(table, row)),
        );

        // read the stored values of the rows another copy includes
        const key = table[TABLE].key;
        const shared = hiding.filter((row) => others.has(Key.name(table, row)));
        const stored = new Map(
            (
                await Snapshot.live(database).select(
                    table,
                    key,
                    shared.map((row) => key.map((name) => row[name])),
                )
            ).map((row) => [Key.name(table, row), row]),
        );

        return rows.map((row, position) => {
            // keep a hidden column as stored while another copy's shape shows it
            const name = Key.name(table, row);
            const shapes = others.get(name) ?? [];
            const values = hidden[position]!.map((column) => [
                column,
                shapes.some((shape) => !shape.includes(column))
                    ? (stored.get(name)?.[column] ?? null)
                    : null,
            ]);

            return { ...row, ...Object.fromEntries(values) };
        });
    }

    /** Read the columns the other copies' shapes hide on some rows, by row key. */
    async #others(
        database: DatabaseConnection,
        table: Table,
        keys: readonly string[],
    ): Promise<Map<string, string[][]>> {
        // read the other copies' records of the rows, a batch at a time
        const name = table[TABLE].sqlName;
        const others = new Map<string, string[][]>();
        for (let start = 0; start < keys.length; start += PRUNE_BATCH) {
            const found = await database
                .select({ key: replicaRow.key, concealed: replicaRow.concealed })
                .from(replicaRow)
                .where(
                    and(
                        not(this.#included()),
                        eq(replicaRow.table, name),
                        inArray(replicaRow.key, keys.slice(start, start + PRUNE_BATCH)),
                    ),
                );
            for (const row of found) {
                others.set(row.key, [...(others.get(row.key) ?? []), row.concealed]);
            }
        }

        return others;
    }

    /** Record the rows the copy includes and the columns its shape hides on each. */
    async #include(
        database: DatabaseConnection,
        table: Table,
        rows: readonly Row[],
        hidden: readonly (readonly string[])[],
    ): Promise<void> {
        const name = table[TABLE].sqlName;
        await database.upsert(
            replicaRow,
            rows.map((row, position) => ({
                name: this.name,
                scope: this.scope,
                table: name,
                key: Key.name(table, row),
                concealed: hidden[position]!,
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
                this.#included(),
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
            await database.remove(
                table,
                batch.filter((key) => !others.has(key)).map((key) => Key.parse(table, key)),
            );

            // clear the columns only this copy's shape showed on the rows the others keep
            const cleared = shown.flatMap(({ key, concealed }) => {
                const shapes = others.get(key) ?? [];
                const columns = (shapes[0] ?? []).filter(
                    (column) =>
                        !concealed.includes(column) &&
                        shapes.every((shape) => shape.includes(column)),
                );

                return columns.length === 0 ? [] : [{ key: Key.parse(table, key), columns }];
            });
            const primary = table[TABLE].key;
            const stored = await Snapshot.live(database).select(
                table,
                primary,
                cleared.map(({ key }) => primary.map((column) => key[column])),
            );
            await database.upsert(
                table,
                stored.map((row) => {
                    const entry = cleared.find(
                        ({ key }) => Key.name(table, key) === Key.name(table, row),
                    )!;

                    return {
                        ...row,
                        ...Object.fromEntries(entry.columns.map((column) => [column, null])),
                    };
                }),
            );
        }
    }

    /** Match the rows the copy includes. */
    #included(): SQL {
        return and(eq(replicaRow.name, this.name), eq(replicaRow.scope, this.scope))!;
    }
}

/** Read a record's home position. */
function recordOrigin(record: typeof replica.$inferSelect): Origin | undefined {
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

/** A group as predictions change it. */
type Predicted = Group & { rows: number };

/** Add a predicted row image to its group, or take it away. */
function predict(node: Node, groups: Map<string, Predicted>, row: Row, sign: 1 | -1): void {
    // find or start the row's group
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

/** Add a JSON value to a sum, or take it away. */
function add(sum: Scalar, value: Scalar, sign: 1 | -1): Scalar {
    return typeof sum === "string" || typeof value === "string"
        ? String(BigInt(sum ?? 0) + BigInt(sign) * BigInt(value ?? 0))
        : Number(sum ?? 0) + sign * Number(value);
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
    /** Whether its tables have the copied shape. */
    readonly isShaped: boolean;
    /** The names of the queries it holds. */
    readonly queries: readonly string[];
    /** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
    readonly confirmedAt?: number;
    /** The staged pages. */
    readonly staged: number;
    /** The aggregate groups it holds. */
    readonly results: number;
}
