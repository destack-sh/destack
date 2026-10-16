import type { Call } from "@destack/sync";
import {
    Change,
    Condition,
    type DatabaseConnection,
    type RowImage,
    type Row,
    Namespace,
    Predicate,
    Scalar,
    Snapshot,
    type Table,
} from "@destack/db";
import type { Controller, Reconciliation } from "@destack/runtime/control";
import { AccessContext } from "@destack/access";
import { RequestId } from "@destack/runtime/request";
import { canonicalize, schema, type JsonValue } from "@destack/schema";
import type { ObjectType } from "./object.ts";
import { Observation, type ObservationReport, type ObservedRecord } from "./condition.ts";
import { type ResultsOf } from "../method/context.ts";
import type { MutateOptions, ObjectStore } from "../server/store.ts";

/** The fields of a pending object's key, as its key name holds them. */
const KeyFields = schema.record(schema.string(), Scalar);

/** The fields grouping objects a controller reconciles together, in JSON form. */
export type KeyFields = Readonly<Record<string, JsonValue>>;

/** A controller declared on an object type: objects with pending work, and their reconciling. */
export interface ObjectController<Object extends ObjectType = ObjectType> {
    /** The objects with work waiting. */
    readonly pending: Condition;
    /** The fields grouping objects reconciled together, the identifier by default. */
    key?(row: RowImage<Object["table"]>): KeyFields;
    /** The tables whose changed rows select keys beside a changed pending object's own. */
    readonly watches?: readonly ObjectWatch[];
    /** Whether a key converges and returns, or follows until its objects leave the pending ones. */
    readonly mode?: "reconcile" | "follow";
    /** The keys reconciled at once, one by default. */
    readonly concurrency?: number;
    /** Reconcile the pending objects of one key, returning the wait until the next look. */
    reconcile(reconciliation: ObjectReconciliation<Object>): Promise<number | undefined>;
}

/** Another table whose changed rows select a controller's keys. */
export interface ObjectWatch<Watched extends Table = Table> {
    /** The watched table. */
    readonly table: Watched;
    /** Read the keys a changed row selects. */
    keys(
        row: RowImage<Watched>,
        database: DatabaseConnection,
    ): readonly KeyFields[] | Promise<readonly KeyFields[]>;
}

/** Watch other tables for a controller. */
export const ObjectWatch = {
    /** Watch a table's changed rows, typing them by the table. */
    of<Watched extends Table>(
        table: Watched,
        keys: ObjectWatch<Watched>["keys"],
    ): ObjectWatch<Watched> {
        return { table, keys };
    },
};

/** One key's reconciliation: pending objects, and methods reconciling them as the store. */
export interface ObjectReconciliation<
    Object extends ObjectType = ObjectType,
> extends Reconciliation {
    /** The pending objects of the key. */
    readonly rows: readonly RowImage<Object["table"]>[];
    /** The reconcile time, in UTC epoch milliseconds. */
    readonly now: number;
    /** The objects' database. */
    readonly database: DatabaseConnection;
    /** The object store running the controller. */
    readonly store: ObjectStore;
    /** The caller the reconciliation acts as: the store's own principal. */
    readonly caller: AccessContext;
    /** Run calls in one transaction as the store under the controller's name, results in order. */
    readonly mutate: <const Calls extends readonly Call[]>(
        calls: Calls,
        options?: Pick<MutateOptions, "transaction" | "scope">,
    ) => Promise<ResultsOf<Calls>>;
    /** Record an object's conditions and observed fields at its generation via observe, once. */
    readonly observe: (
        object: ObjectType,
        row: ObservedRecord & Readonly<Record<string, unknown>>,
        observation: ObservationReport,
    ) => Promise<unknown>;
}

/** Build the controllers object types declare. */
export const ObjectController = {
    /** Build a type's declared controller, reconciling or following pending objects by key. */
    control(store: ObjectStore, object: ObjectType, declared: ObjectController): Controller {
        // key each pending object by its declared fields
        const table = object.table;
        const match = Predicate.compile(
            Condition.resolve(declared.pending, Namespace.fields(table)),
            table,
        );
        const watches = new Map((declared.watches ?? []).map((watch) => [watch.table, watch]));
        const readKey = (row: Row): KeyFields =>
            declared.key?.(row) ?? { id: ObjectController.readIdentifier(row) };
        const keys = async (change: Change) => {
            // select the keys a watch of the changed table reads
            const watch = watches.get(change.table);
            const watched =
                watch === undefined ? [] : await watch.keys(Change.image(change), store.database);

            // add the key of a changed object left pending
            const row = Change.image(change);
            const isPending =
                Change.of(change, table) &&
                change.operation !== "delete" &&
                Predicate.matches(match, row);
            const selected = isPending ? [...watched, readKey(row)] : watched;

            return [...new Set(selected.map((fields) => canonicalize(fields)))];
        };

        return {
            name: object.name,
            watches: [...new Set([table, ...watches.keys()])],
            ...(declared.concurrency === undefined ? {} : { concurrency: declared.concurrency }),
            ...(declared.mode === undefined ? {} : { mode: declared.mode }),
            keys,
            list: async () => {
                // list the keys of every pending object
                const rows = await Snapshot.live(store.database)
                    .read(table)
                    .findMany({ where: declared.pending });

                return [...new Set(rows.map((row) => canonicalize(readKey(row))))];
            },
            reconcile: (key, reconciliation) =>
                ObjectController.reconcilePending(store, object, declared, key, reconciliation),
        };
    },

    /** Read an object row's identifier, which every object table has. */
    readIdentifier(row: Row): string {
        return schema.string().parse(row["id"]);
    },

    /** Reconcile one key's pending objects, if any are left. */
    async reconcilePending(
        store: ObjectStore,
        object: ObjectType,
        declared: ObjectController,
        key: string,
        reconciliation: Reconciliation,
    ): Promise<number | undefined> {
        // read the key's pending objects
        const pending: Condition = {
            AND: [declared.pending, Condition.equal(KeyFields.parse(JSON.parse(key)))],
        };
        const rows = await Snapshot.live(store.database)
            .read(object.table)
            .findMany({ where: pending });
        if (rows.length === 0) {
            return undefined;
        }

        // reconcile them, running methods as the store's principal at one time
        const now = store.clock.now();
        const caller = store.self;
        const mutate: ObjectReconciliation["mutate"] = (calls, options) =>
            store.mutate(
                caller,
                { id: RequestId.create(now), calls: [...calls] },
                { ...options, component: object.name },
            );

        return declared.reconcile({
            rows,
            now,
            database: store.database,
            store,
            caller,
            signal: reconciliation.signal,
            ...(reconciliation.epoch === undefined ? {} : { epoch: reconciliation.epoch }),
            changed: () => reconciliation.changed(),
            mutate,
            observe: async (observed, row, observation) => {
                // skip an observation the row has
                if (Observation.isRecorded(row, observation)) {
                    return row;
                }

                // record the generation, the conditions and the fields
                const [written] = await mutate([Observation.call(observed, row, observation)]);

                return written;
            },
        });
    },
};
