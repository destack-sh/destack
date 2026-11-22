import {
    and,
    eq,
    isNotNull,
    inArray,
    Key,
    or,
    sql,
    TABLE,
    type Dialect,
    type SQL,
    type SQLWrapper,
    Statement,
    listed,
    DatabaseError,
    type Row,
    type Select,
    Table,
    Change,
    LogPosition,
} from "@destack/db";
import type { DatabaseConnection, Snapshot } from "@destack/db/database";
import { SyncError } from "../error/error.ts";
import { schema, canonicalize, found } from "@destack/schema";
import { PublicationAudience, type SubscriptionRecord } from "./publication.ts";
import { Subject } from "../call/subject.ts";
import { scopeTable } from "../scope/table.ts";
import { untilAborted } from "@destack/environment";
import {
    replica,
    replicaScope,
    type Reflection,
    replicaPosition,
    replicaSubscription,
    replicaKey,
} from "./table.ts";

/** The scopes each of a replica's tables' rows live in, by the table's SQL name. */
type ReplicaLiving = Record<string, string[]>;

/** A replica's position row without its key. */
export type ReplicaPosition = Omit<Select<typeof replicaPosition>, "name" | "scope">;

/** A replica's watermark: the home position applied, and when its page arrived. */
const ReplicaMark = schema.object({
    /** The home position the replica applied. */
    position: LogPosition,
    /** When the page arrived, in UTC epoch milliseconds. */
    receivedAt: schema.number(),
});
/** A replica's watermark: the home position applied, and when its page arrived. */
type ReplicaMark = schema.Infer<typeof ReplicaMark>;

/** The layout digest of each replicated table, by SQL name. */
export type Layouts = Readonly<Record<string, string>>;

/** A replica's registration with its position and subscription. */
export type ReplicaRecord = Select<typeof replica> & {
    /** The scopes each replicated table's rows live in, by SQL name. */
    readonly living: ReplicaLiving;
} & Omit<Select<typeof replicaPosition>, "name" | "scope"> & {
        /** The subscription it follows, absent for a replica without one. */
        readonly subscription: SubscriptionRecord | undefined;
        /** The audience its subscription is decided for, absent without a subscription. */
        readonly audience: PublicationAudience | undefined;
    };

/** Join a replica's registration to its subscription, by the replica's name and scope. */
const SUBSCRIBED = and(
    eq(replicaSubscription.name, replica.name),
    eq(replicaSubscription.scope, replica.scope),
);

/** Join a replica's registration to its position, by the replica's name and scope. */
export const POSITIONED = and(
    eq(replicaPosition.name, replica.name),
    eq(replicaPosition.scope, replica.scope),
);

/** A calling principal's fields as values or bound operands. */
interface SubjectOperands {
    /** The principal's package. */
    readonly packageId: Subject["packageId"] | SQLWrapper;
    /** The principal's type. */
    readonly type: string | SQLWrapper;
    /** The principal's scope. */
    readonly scope: string | SQLWrapper;
    /** The principal's identifier. */
    readonly id: string | SQLWrapper;
}

/** A replica with the position of its home's log it has applied. */
export interface ReplicaHome {
    /** The replica's name. */
    readonly name: string;
    /** The scope the replica replicates. */
    readonly scope: string;
    /** The position of its home's log, absent before the replica recorded one. */
    readonly home: LogPosition | undefined;
}

/** The columns a replica's table listing reads. */
const STORED = {
    name: replicaScope.name,
    scope: replicaScope.scope,
    table: replicaScope.table,
    living: replicaScope.living,
};

/** Build the listing of replicas' tables: every replica's, or those a reader or principal sees. */
function tablesRead(seen: "every" | "readers" | "callers") {
    return new Statement((value, database) => {
        // read every replica's tables in a stable order
        const query = database.select(STORED).from(replicaScope);
        if (seen === "every") {
            return query.orderBy(...Object.values(STORED));
        }

        // read those the reader or the principal sees by their subscriptions
        return query
            .innerJoin(
                replicaSubscription,
                and(
                    eq(replicaSubscription.name, replicaScope.name),
                    eq(replicaSubscription.scope, replicaScope.scope),
                ),
            )
            .where(ReplicaSet.seen(value, seen === "callers"))
            .orderBy(...Object.values(STORED));
    });
}

/** The listings of replicas' tables. */
const REPLICA_TABLES = {
    every: tablesRead("every"),
    readers: tablesRead("readers"),
    callers: tablesRead("callers"),
};

/** Build the read of a table's keys the replicas a reader or a principal sees include. */
function keysRead(isCaller: boolean) {
    return new Statement((value, database) =>
        database
            .select({ key: replicaKey.key })
            .from(replicaKey)
            .innerJoin(
                replicaSubscription,
                and(
                    eq(replicaSubscription.name, replicaKey.name),
                    eq(replicaSubscription.scope, replicaKey.scope),
                ),
            )
            .where(
                and(
                    eq(replicaKey.table, value("table")),
                    listed(replicaKey.key, value("keys")),
                    ReplicaSet.seen(value, isCaller),
                ),
            ),
    );
}

/** The reads of the keys replicas include. */
const REPLICA_KEYS = {
    readers: keysRead(false),
    callers: keysRead(true),
};

/** Read a replica's stored record. */
export const ReplicaRecord = {
    /** Decide whether a record has a position, its home position included. */
    isAt(record: ReplicaRecord | undefined, position: ReplicaPosition): boolean {
        return (
            record !== undefined &&
            record.log === position.log &&
            record.epoch === position.epoch &&
            record.sequence === position.sequence &&
            record.originLog === position.originLog &&
            record.originEpoch === position.originEpoch &&
            record.originSequence === position.originSequence
        );
    },

    /** Read a record's home position, absent before its first snapshot. */
    origin(record: Select<typeof replicaPosition>): LogPosition | undefined {
        const { position, origin } = ReplicaRecord.positions(record);

        return origin ?? position;
    },

    /** Read a record's source position and its relaying source's home position, when recorded. */
    positions(record: Select<typeof replicaPosition>): {
        readonly position?: LogPosition;
        readonly origin?: LogPosition;
    } {
        const { log, epoch, sequence, originLog, originEpoch, originSequence } = record;

        return {
            ...(log === null || epoch === null || sequence === null
                ? {}
                : { position: { log, epoch, sequence } }),
            ...(originLog === null || originEpoch === null || originSequence === null
                ? {}
                : { origin: { log: originLog, epoch: originEpoch, sequence: originSequence } }),
        };
    },
};

/** The replicas a database stores: their records, positions, marks and subscriptions. */
export class ReplicaSet {
    /** The database the replicas live in. */
    readonly database: DatabaseConnection;

    /** Read the replicas of a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Match the rows of a table keyed by a text `id` in the replicas a reader sees, as SQL. */
    static includes(table: Table, target: string, caller: Subject | undefined): SQL {
        // match the row's key in the replicas the reader sees
        const sqlName = table[TABLE].sqlName;
        const key = sql`'["' || ${sqlName} || '","' || ${table[TABLE].column("id")} || '"]'`;
        const seen = ReplicaSet.#seen(target, caller);

        return sql`EXISTS (SELECT 1 FROM ${replicaKey} INNER JOIN ${replicaSubscription} ON ${replicaSubscription.name} = ${replicaKey.name} AND ${replicaSubscription.scope} = ${replicaKey.scope} WHERE ${replicaKey.table} = ${sqlName} AND ${replicaKey.key} = ${key} AND ${seen})`;
    }

    /** List the keys of some rows of a table that the replicas a reader sees include. */
    async keysIncluded(
        table: Table,
        rows: readonly Row[],
        target: string,
        caller: Subject | undefined,
    ): Promise<ReadonlySet<string>> {
        // bind the rows' keys and the reader, then read the included ones
        const { database } = this;
        const values = {
            table: table[TABLE].sqlName,
            keys: JSON.stringify(rows.map((row) => Key.name(table, row))),
            target,
            ...ReplicaSet.#subject(caller),
        };
        const included = await (
            caller === undefined ? REPLICA_KEYS.readers : REPLICA_KEYS.callers
        ).select(database, values);

        return new Set(included.map((row) => row.key));
    }

    /** Match the replicas a reader of a scope sees: decided for its readers, or the caller. */
    static #seen(target: string | SQLWrapper, caller: SubjectOperands | undefined): SQL {
        // match the replicas decided for the scope's readers, and those decided for the calling principal anywhere
        const isTarget = eq(replicaSubscription.target, target);
        const isReader = and(isTarget, eq(replicaSubscription.audience, "reader"));
        const isCaller =
            caller === undefined
                ? undefined
                : and(
                      eq(replicaSubscription.audience, "caller"),
                      eq(replicaSubscription.subjectPackageId, caller.packageId),
                      eq(replicaSubscription.subjectType, caller.type),
                      eq(replicaSubscription.subjectScope, caller.scope),
                      eq(replicaSubscription.subjectId, caller.id),
                  );

        return or(isReader, isCaller);
    }

    /** Bind a calling principal's fields by name, none for a reader alone. */
    static #subject(caller: Subject | undefined): Readonly<Record<string, string>> {
        return caller === undefined
            ? {}
            : {
                  packageId: caller.packageId,
                  type: caller.type,
                  scope: caller.scope,
                  id: caller.id,
              };
    }

    /** Match the replicas a reader or a calling principal sees, its operands bound by name. */
    static seen(value: (name: string) => SQLWrapper, isCaller: boolean): SQL {
        return ReplicaSet.#seen(
            value("target"),
            isCaller
                ? {
                      packageId: value("packageId"),
                      type: value("type"),
                      scope: value("scope"),
                      id: value("id"),
                  }
                : undefined,
        );
    }

    /** Take rows a database pruned out of every replica including them. */
    async release(table: Table, rows: readonly Row[]): Promise<void> {
        // write no bookkeeping in a database without replicas
        const { database } = this;
        if (!database.tables.includes(replicaKey) || rows.length === 0) {
            return;
        }

        // delete the rows' entries of every replica
        await database.delete(replicaKey).where(
            and(
                eq(replicaKey.table, table[TABLE].sqlName),
                inArray(
                    replicaKey.key,
                    rows.map((row) => Key.name(table, row)),
                ),
            ),
        );
    }

    /** List the replicas' tables by the scope their rows live in, or those a reader sees. */
    async tables(
        ...reader: [] | [target: string, caller: Subject | undefined]
    ): Promise<readonly { readonly scope: string; readonly tables: readonly string[] }[]> {
        // list no replicas in a database without them
        const { database } = this;
        if (!database.tables.includes(replica)) {
            return [];
        }

        // read what every replica stores, or those a reader sees by their subscriptions
        const [target, caller] = reader;
        const rows =
            target === undefined
                ? await REPLICA_TABLES.every.select(database)
                : await (
                      caller === undefined ? REPLICA_TABLES.readers : REPLICA_TABLES.callers
                  ).select(database, { target, ...ReplicaSet.#subject(caller) });

        // list each replica's tables by the scopes their rows live in
        const byReplica = new Map<string, { scope: string; tables: string[] }>();
        for (const row of rows) {
            const key = `${row.name} ${row.scope} ${row.living}`;
            const entry = byReplica.get(key) ?? { scope: row.living, tables: [] };
            entry.tables.push(row.table);
            byReplica.set(key, entry);
        }

        return [...byReplica.values()];
    }

    /** List the scopes whose rows of a table the replicas have, or those a reader sees. */
    async scopes(
        table: Table,
        ...reader: [] | [target: string, caller: Subject | undefined]
    ): Promise<string[]> {
        // take the scopes of the replicas with the table
        const sqlName = table[TABLE].sqlName;
        const records = await this.tables(...reader);

        return [
            ...new Set(
                records
                    .filter((record) => record.tables.includes(sqlName))
                    .map(({ scope }) => scope),
            ),
        ];
    }

    /** List the home position and tables of each replica past its first snapshot. */
    async reflections(): Promise<Reflection[]> {
        const { database } = this;
        const records = await database
            .select({ tables: replica.tables, position: replicaPosition })
            .from(replica)
            .innerJoin(replicaPosition, POSITIONED);

        return records.flatMap(({ tables, position }) => {
            const origin = ReplicaRecord.origin(position);

            return origin === undefined ? [] : [{ position: origin, tables }];
        });
    }

    /** Match a log, or a log at an epoch, that some replica here reflects. */
    static reflects(log: SQLWrapper, epoch?: SQLWrapper): SQL {
        // reflect the relaying source's home position, else the source's own
        const isRelayed = isNotNull(replicaPosition.originLog);
        const reflectedLog = sql`CASE WHEN ${isRelayed} THEN ${replicaPosition.originLog} ELSE ${replicaPosition.log} END`;
        const reflectedEpoch = sql`CASE WHEN ${isRelayed} THEN ${replicaPosition.originEpoch} ELSE ${replicaPosition.epoch} END`;
        const isAtEpoch = epoch === undefined ? sql`TRUE` : sql`${reflectedEpoch} = ${epoch}`;

        return sql`EXISTS (SELECT 1 FROM ${replica} INNER JOIN ${replicaPosition} ON ${POSITIONED} WHERE ${reflectedLog} = ${log} AND ${isAtEpoch})`;
    }

    /** Report whether a database replicates a scope, or the scope's rows of one table. */
    static async isReplicated(
        snapshot: Pick<Snapshot, "read">,
        scope: string,
        table?: Table,
    ): Promise<boolean> {
        // read one replica's row of the scope's rows of the table, or of any table
        const name = table?.[TABLE].sqlName;
        const stored = await snapshot.read(replicaScope).findFirst({
            where: { living: scope, ...(name === undefined ? {} : { table: name }) },
        });

        return stored !== undefined;
    }

    /** Wait until the replicas including a scope reflect its home up to a position. */
    async wait(scope: string, position: LogPosition, signal: AbortSignal): Promise<boolean> {
        // read the replicas including the scope, again at each mark while none does
        let replicas = await this.including(scope);

        return await this.waitMarked(async () => {
            if (replicas.length === 0) {
                replicas = await this.including(scope);
            }

            return this.isAt(replicas, position);
        }, signal);
    }

    /** Wait until some replicas arrived at a position, as their marks show. */
    async waitAt(
        replicas: readonly { readonly name: string; readonly scope: string }[],
        position: LogPosition,
        signal: AbortSignal,
    ): Promise<boolean> {
        return await this.waitMarked(() => this.isAt(replicas, position), signal);
    }

    /** Check now and at each mark change, until the check passes or the signal aborts. */
    async waitMarked(check: () => Promise<boolean>, signal: AbortSignal): Promise<boolean> {
        const { database } = this;
        while (!signal.aborted) {
            // watch the marks from before the check, so no change passes unseen
            const changed = Promise.withResolvers<void>();
            const unwatch = database.tracker.watchMarked(() => changed.resolve());
            try {
                if (await check()) {
                    return true;
                }
                await untilAborted(changed.promise, signal);
            } finally {
                unwatch();
            }
        }

        return false;
    }

    /** Report whether some replicas arrived at a position, skipping those of other logs. */
    async isAt(
        replicas: readonly { readonly name: string; readonly scope: string }[],
        position: LogPosition,
    ): Promise<boolean> {
        // require each replica following the position's log to have reached it, and one at least
        const homes = await this.homes(replicas);
        const following = homes.filter(
            ({ home }) => home === undefined || home.log === position.log,
        );

        return following.length > 0 && ReplicaSet.late(homes, position).length === 0;
    }

    /** Read each replica's home position: its mark, or the one it recorded before any mark. */
    async homes(
        replicas: readonly { readonly name: string; readonly scope: string }[],
    ): Promise<ReplicaHome[]> {
        return await Promise.all(
            replicas.map(async ({ name, scope }) => ({
                name,
                scope,
                home: this.marked(name, scope)?.position ?? (await this.origin(name, scope)),
            })),
        );
    }

    /** List the replicas yet to reach a position of the log they follow, or of none yet. */
    static late(homes: readonly ReplicaHome[], position: LogPosition): ReplicaHome[] {
        return homes.filter(
            ({ scope, home }) =>
                (home === undefined || home.log === position.log) &&
                !ReplicaSet.#isReached(home, scope, position),
        );
    }

    /** List the replicas including a scope's rows, by name and the scope each replicates. */
    async including(scope: string): Promise<{ readonly name: string; readonly scope: string }[]> {
        const including = await this.#recordsIncluding([scope]);

        return including.map(({ record }) => ({ name: record.name, scope: record.scope }));
    }

    /** Report whether a home position arrived at a position of its log, refusing older history. */
    static #isReached(
        origin: LogPosition | undefined,
        scope: string,
        position: LogPosition,
    ): boolean {
        // arrive nowhere before following the position's log
        if (origin === undefined || origin.log !== position.log) {
            return false;
        }
        // refuse a position of an outdated history
        else if (origin.epoch > position.epoch) {
            throw DatabaseError.of(
                "EPOCH_CONFLICT",
                `cannot follow '${scope}' from the position: it started a new epoch after it`,
            );
        }
        // arrive at the position within its epoch
        else {
            return origin.epoch === position.epoch && origin.sequence >= position.sequence;
        }
    }

    /** Report whether a database's replica of a scope is a complete snapshot, as informers sync. */
    async isSynced(name: string, scope: string): Promise<boolean> {
        const record = await this.find(name, scope);

        return record !== undefined && ReplicaRecord.origin(record) !== undefined;
    }

    /** Report whether a database's replica completed a run of exactly a subscription. */
    async isCurrent(subscription: SubscriptionRecord): Promise<boolean> {
        const recorded = await this.subscription(subscription.name, subscription.scope);

        return (
            recorded !== undefined &&
            canonicalize(recorded) === canonicalize(ReplicaSet.subscribed(subscription))
        );
    }

    /** Read the subscription a database's replica completed a run of, absent before its first. */
    async subscription(name: string, scope: string): Promise<SubscriptionRecord | undefined> {
        return (await this.find(name, scope))?.subscription;
    }

    /** Read a replica's home position, absent before its first snapshot. */
    async origin(name: string, scope: string): Promise<LogPosition | undefined> {
        const record = await this.find(name, scope);

        return record === undefined ? undefined : ReplicaRecord.origin(record);
    }

    /** Read a replica's home position as of a log position, absent once its rows moved since. */
    async originAt(
        followed: { readonly name: string; readonly scope: string },
        position: LogPosition,
    ): Promise<LogPosition | undefined> {
        const { database } = this;
        return await database.transaction(
            async (transaction) => {
                // read the replica's position writes since the position, in its log and epoch
                const head = await transaction.log.position();
                const isSameEpoch = head.log === position.log && head.epoch === position.epoch;
                const since = isSameEpoch
                    ? await transaction.log.read({
                          tables: [replicaPosition],
                          after: position.sequence,
                          scopes: [followed.scope],
                      })
                    : undefined;

                // read the origin while the replica's rows stayed as of the position
                const isMoved =
                    since === undefined ||
                    since.changes.some((change) => Change.image(change)["name"] === followed.name);

                return isMoved
                    ? undefined
                    : await new ReplicaSet(transaction).origin(followed.name, followed.scope);
            },
            { isReadOnly: true },
        );
    }

    /** Read a database's record of a named replica of a scope, absent before it follows. */
    async find(name: string, scope: string): Promise<ReplicaRecord | undefined> {
        // read the replica's registration with its position, its subscription and its living scopes
        const { database } = this;
        const [record] = await database
            .select({
                registration: replica,
                position: replicaPosition,
                subscription: replicaSubscription,
                living: ReplicaSet.#livingText(database.dialect),
            })
            .from(replica)
            .innerJoin(replicaPosition, POSITIONED)
            .leftJoin(replicaSubscription, SUBSCRIBED)
            .where(and(eq(replica.name, name), eq(replica.scope, scope)));

        return record === undefined
            ? undefined
            : ReplicaSet.#merge(
                  record.registration,
                  record.position,
                  record.subscription,
                  ReplicaSet.#readLiving(record.living),
              );
    }

    /** Refuse relaying a replica that has no position yet. */
    async requireRelayable(name: string, scope: string): Promise<void> {
        const record = await this.find(name, scope);
        if (record !== undefined && ReplicaRecord.origin(record) === undefined) {
            throw SyncError.of(
                "REPLICA_UNAVAILABLE",
                `replica of '${scope}' is unavailable: it has no position yet`,
            );
        }
    }

    /** Wait until a publication's replicas of some scopes are live, answering one still stale. */
    async waitLive(
        publication: string,
        scopes: readonly string[],
        lag: number,
        signal: AbortSignal,
    ): Promise<string | undefined> {
        let stale: string | undefined;
        await this.waitMarked(async () => {
            stale = await this.stale(publication, scopes, lag);

            return stale === undefined;
        }, signal);

        return stale;
    }

    /** Find the first scope a replica includes while it is not live or has no page yet. */
    async stale(
        publication: string,
        scopes: readonly string[],
        lag: number,
    ): Promise<string | undefined> {
        // take the replicas of the publication including each scope
        const records = await this.#recordsIncluding(scopes);
        const silent = records.filter(
            ({ record }) =>
                record.subscription?.publication === publication &&
                !this.isLive(record.name, record.scope, lag),
        );

        return scopes.find((scope) => silent.some((each) => each.scope === scope));
    }

    /** Report whether a replica is live: a follow on any instance received a page within a lag. */
    isLive(name: string, scope: string, lag: number): boolean {
        const { database } = this;
        const marked = this.marked(name, scope);

        return marked !== undefined && database.state.clock.now() - marked.receivedAt < lag;
    }

    /** Mark a replica with the home position a page brought, announced at once. */
    mark(followed: { readonly name: string; readonly scope: string }, position: LogPosition): void {
        const { database } = this;
        const marked: ReplicaMark = { position, receivedAt: database.state.clock.now() };
        database.tracker.mark(ReplicaSet.owner(followed.name, followed.scope), marked);
    }

    /** Refresh a replica's mark at a beat, announced with the tracker's heartbeat. */
    refresh(
        followed: { readonly name: string; readonly scope: string },
        position: LogPosition,
    ): void {
        const { database } = this;
        const marked: ReplicaMark = { position, receivedAt: database.state.clock.now() };
        database.tracker.refresh(ReplicaSet.owner(followed.name, followed.scope), marked);
    }

    /** Read a replica's latest mark across the live instances, absent before any. */
    marked(name: string, scope: string): ReplicaMark | undefined {
        // take the mark that arrived last across the instances
        const { database } = this;
        const marks = database.tracker
            .marks(ReplicaSet.owner(name, scope))
            .map((mark) => ReplicaMark.parse(mark));

        return marks.reduce<ReplicaMark | undefined>(
            (latest, mark) =>
                latest === undefined || mark.receivedAt > latest.receivedAt ? mark : latest,
            undefined,
        );
    }

    /** Tell a listener each time a replica's marks change, until stopped. */
    watchMarked(
        followed: { readonly name: string; readonly scope: string },
        listener: () => void,
    ): () => void {
        const { database } = this;
        const owner = ReplicaSet.owner(followed.name, followed.scope);

        return database.tracker.watchMarked((marked) => {
            if (marked === owner) {
                listener();
            }
        });
    }

    /** Build the owner key a database's tracker marks a replica under. */
    static owner(name: string, scope: string): string {
        return `replica/${name}/${scope}`;
    }

    /** The read of every replica with the rows of some scopes it includes. */
    static readonly #including = new Statement((value, database) =>
        database
            .select({
                replica,
                replica_position: replicaPosition,
                replica_subscription: replicaSubscription,
                replica_key: replicaKey,
                living: ReplicaSet.#livingText(database.dialect),
            })
            .from(replica)
            .innerJoin(replicaPosition, POSITIONED)
            .leftJoin(replicaSubscription, SUBSCRIBED)
            .leftJoin(
                replicaKey,
                and(
                    eq(replicaKey.name, replica.name),
                    eq(replicaKey.scope, replica.scope),
                    eq(replicaKey.table, scopeTable[TABLE].sqlName),
                    listed(replicaKey.key, value("keys")),
                ),
            ),
    );

    /** Read the records of the replicas including some scopes, once per scope and replica. */
    async #recordsIncluding(
        scopes: readonly string[],
    ): Promise<{ readonly scope: string; readonly record: ReplicaRecord }[]> {
        // read every replica with the scopes' rows it includes, a database having a few replicas
        const { database } = this;
        const keys = new Map(scopes.map((scope) => [Key.name(scopeTable, { scope }), scope]));
        const rows = await ReplicaSet.#including.select(database, {
            keys: JSON.stringify([...keys.keys()]),
        });

        // pair each replica with the scopes it replicates and the scopes whose rows it includes
        const pairs = new Map<string, { scope: string; record: ReplicaRecord }>();
        for (const {
            replica: registration,
            replica_position: position,
            replica_subscription: subscription,
            replica_key: included,
            living,
        } of rows) {
            const record = ReplicaSet.#merge(
                registration,
                position,
                subscription,
                ReplicaSet.#readLiving(living),
            );
            const kept = [
                ...ReplicaSet.#livingScopes(record).filter((scope) => scopes.includes(scope)),
                ...(included === null ? [] : [found(keys, included.key)]),
            ];
            for (const scope of kept) {
                pairs.set(`${scope} ${record.name} ${record.scope}`, { scope, record });
            }
        }

        return [...pairs.values()];
    }

    /** Select a replica's living scopes beside its row, as JSON text of table and scope pairs. */
    static #livingText(dialect: Dialect): SQL<string> {
        const of = and(eq(replicaScope.name, replica.name), eq(replicaScope.scope, replica.scope));

        return dialect === "sqlite"
            ? sql<string>`(SELECT json_group_array(json_array(${replicaScope.table}, ${replicaScope.living})) FROM ${replicaScope} WHERE ${of})`
            : sql<string>`(SELECT coalesce(json_agg(json_build_array(${replicaScope.table}, ${replicaScope.living})), '[]'::json)::text FROM ${replicaScope} WHERE ${of})`;
    }

    /** Read a replica's living scopes from their JSON text, each table's scopes sorted. */
    static #readLiving(stored: string): ReplicaLiving {
        // read the pairs, then list each table's scopes in order
        const pairs = schema
            .array(schema.tuple([schema.string(), schema.string()]))
            .parse(JSON.parse(stored));
        const living: ReplicaLiving = {};
        for (const [table, scope] of pairs.toSorted(([, left], [, right]) =>
            left.localeCompare(right),
        )) {
            living[table] = [...(living[table] ?? []), scope];
        }

        return living;
    }

    /** Sort each table's living scopes and leave out a table without any, as stored rows read. */
    static sortedLiving(living: ReplicaLiving): ReplicaLiving {
        return Object.fromEntries(
            Object.entries(living)
                .filter(([, scopes]) => scopes.length > 0)
                .map(([table, scopes]) => [
                    table,
                    scopes.toSorted((left, right) => left.localeCompare(right)),
                ]),
        );
    }

    /** List the scopes a replica's rows live in: its own and those its tables' rows live in. */
    static #livingScopes(record: Pick<ReplicaRecord, "scope" | "living">): string[] {
        return [...new Set([record.scope, ...Object.values(record.living).flat()])];
    }

    /** Decide whether every table a record and the replica both have keeps its layout. */
    static isLaidOut(recorded: Layouts | null, layouts: Layouts): boolean {
        return (
            recorded !== null &&
            Object.entries(recorded).every(
                ([table, digest]) => layouts[table] === undefined || layouts[table] === digest,
            )
        );
    }

    /** List the subscriptions of a database's replicas, leaving out replicas without one. */
    static async subscriptions(reader: Pick<Snapshot, "read">): Promise<SubscriptionRecord[]> {
        const rows = await reader.read(replicaSubscription).findMany();

        return rows.map((row) => ReplicaSet.subscriptionOf(row));
    }

    /** Write a subscription as its replica's subscription columns. */
    static subscriptionColumns(subscription: SubscriptionRecord, audience: PublicationAudience) {
        const { subject } = subscription;

        return {
            publication: subscription.publication,
            target: subscription.target,
            subjectPackageId: subject?.packageId ?? null,
            subjectType: subject?.type ?? null,
            subjectScope: subject?.scope ?? null,
            subjectId: subject?.id ?? null,
            subjectRelation: subject?.relation ?? null,
            parameters: subscription.parameters,
            audience,
        };
    }

    /** Read the subscription a replica's subscription row stores. */
    static subscriptionOf(row: Select<typeof replicaSubscription>): SubscriptionRecord {
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
            publication: row.publication,
            scope: row.scope,
            target: row.target,
            ...(subject === undefined ? {} : { subject }),
            parameters: row.parameters,
        };
    }

    /** Pick what a subscription follows, without where it resumes or its revision. */
    static subscribed(subscription: SubscriptionRecord): SubscriptionRecord {
        const { name, publication, scope, target, subject, parameters } = subscription;

        return {
            name,
            publication,
            scope,
            target,
            ...(subject === undefined ? {} : { subject }),
            parameters,
        };
    }

    /** Merge a replica's registration with its position and subscription. */
    static #merge(
        registration: Select<typeof replica>,
        position: Select<typeof replicaPosition>,
        subscription: Select<typeof replicaSubscription> | null,
        living: ReplicaLiving,
    ): ReplicaRecord {
        return {
            ...position,
            ...registration,
            living,
            subscription:
                subscription === null ? undefined : ReplicaSet.subscriptionOf(subscription),
            audience:
                subscription === null
                    ? undefined
                    : PublicationAudience.parse(subscription.audience),
        };
    }
}
