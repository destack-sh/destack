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
    type ReconcileController,
} from "@destack/db";
import type { Reconciliation } from "@destack/db";
import { AccessContext } from "@destack/access";
import { RequestId } from "@destack/sync";
import { Failure } from "@destack/error";
import { canonicalize, schema, type JsonValue } from "@destack/schema";
import type { ObjectType } from "./object.ts";
import {
    ConditionMap,
    Observation,
    type ObservationReport,
    type ObservedRecord,
} from "./condition.ts";
import { type ResultsOf } from "../method/context.ts";
import type { MutateOptions, ObjectStore } from "../server/store.ts";
import { CallSite } from "../method/site.ts";

/** The condition of a controlled object's last reconcile succeeding, as Crossplane's Synced. */
const SYNCED = "Synced";

/** The synced condition of a controlled object whose last reconcile succeeded. */
const SYNCED_CONDITION = { status: "true", reason: "ReconcileSuccess", message: "" } as const;

/** The fields of a controlled object its controller observes. */
const ObservedFields = schema
    .object({
        /** The scope the object lives in. */
        scope: schema.string(),
        /** The object's identifier. */
        id: schema.string(),
        /** The desired generation. */
        generation: schema.number().int(),
        /** The conditions by name. */
        conditions: ConditionMap,
    })
    .strip();

/** The fields of a pending object's key, as its key name has them. */
const KeyFields = schema.record(schema.string(), Scalar);

/** The fields grouping objects a controller reconciles together, in JSON form. */
export type KeyFields = Readonly<Record<string, JsonValue>>;

/** A controller declared on an object type: objects with pending work, and their reconciling. */
export interface ObjectController<Object extends ObjectType = ObjectType> {
    /** The objects with work waiting. */
    readonly pending: Condition;
    /** The fields grouping objects reconciled together, the identifier by default. */
    key?(row: RowImage<Object["table"]>): KeyFields;
    /** The tables whose changed rows select keys beside a changed pending object's key. */
    readonly watches?: readonly ObjectWatch[];
    /** The keys reconciled at once, one by default. */
    readonly concurrency?: number;
    /** List the keys a start reconciles with a separate query; changes still match `pending`. */
    readonly list?: (view: Snapshot) => Promise<readonly KeyFields[]>;
    /** Reconcile the pending objects of one key, returning the wait until the next look. */
    reconcile(reconciliation: ObjectReconciliation<Object>): Promise<number | undefined>;
}

/** Another table whose changed rows select a controller's keys. */
export interface ObjectWatch<Watched extends Table = Table> {
    /** The watched table. */
    readonly table: Watched;
    /** Read the keys a changed row selects, given the change with its row before and after. */
    keys(
        row: RowImage<Watched>,
        database: DatabaseConnection,
        change: Change<Watched>,
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
    /** The caller the reconciliation acts as: the store's principal. */
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
    /** Build a type's declared controller, reconciling pending objects by key. */
    control(
        store: ObjectStore,
        object: ObjectType,
        declared: ObjectController,
    ): ReconcileController {
        // key each pending object by its declared fields
        const table = object.table;
        const match = Predicate.compile(
            Condition.resolve(declared.pending, Namespace.fields(table)),
            table,
        );
        const watches = Map.groupBy(declared.watches ?? [], (watch) => watch.table);
        const readKey = (row: Row): KeyFields =>
            declared.key?.(row) ?? { id: ObjectController.readIdentifier(row) };
        const keys = async (change: Change) => {
            // select the keys each watch of the changed table reads
            const watched = (
                await Promise.all(
                    (watches.get(change.table) ?? []).map(async (watch) =>
                        watch.keys(Change.image(change), store.database, change),
                    ),
                )
            ).flat();

            // add the key of a changed object left pending, past a controlled object's status change
            const row = Change.image(change);
            const isPending =
                Change.of(change, table) &&
                change.operation !== "delete" &&
                !ObjectController.isObserved(object, change) &&
                Predicate.matches(match, row);
            const selected = isPending ? [...watched, readKey(row)] : watched;

            return [...new Set(selected.map((fields) => canonicalize(fields)))];
        };

        return {
            name: object.name,
            watches: [...new Set([table, ...watches.keys()])],
            ...(declared.concurrency === undefined ? {} : { concurrency: declared.concurrency }),
            keys,
            list: async (view) => {
                // list the declared query's keys, or those of every pending object
                const listed =
                    declared.list === undefined
                        ? (await view.read(table).findMany({ where: declared.pending })).map(
                              readKey,
                          )
                        : await declared.list(view);

                return [...new Set(listed.map((fields) => canonicalize(fields)))];
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

        // reconcile them as the type's serving principal and implementing package, at one time
        const now = store.clock.now();
        const caller = store.as(object, now);
        const site = CallSite.package(store.implementer(object));
        const mutate: ObjectReconciliation["mutate"] = (calls, options) =>
            store.mutate(
                caller,
                { id: RequestId.create(store.environment), calls: [...calls] },
                { ...options, component: object.name, site },
            );
        const observe: ObjectReconciliation["observe"] = async (observed, row, observation) => {
            // skip an observation the row has
            if (Observation.isRecorded(row, observation)) {
                return row;
            }

            // record the generation, the conditions and the fields
            const [written] = await mutate([Observation.call(observed, row, observation)]);

            return written;
        };
        const observeAll = async (
            observed: readonly (ObservedRecord & Readonly<Record<string, unknown>>)[],
            observation: ObservationReport,
        ) => {
            // record one observation on every row without it, in one mutation
            const calls = observed
                .filter((row) => !Observation.isRecorded(row, observation))
                .map((row) => Observation.call(object, row, observation));
            if (calls.length > 0) {
                await mutate(calls);
            }
        };

        // reconcile, recording a lasting failure on each controlled object until its next change
        try {
            const delay = await declared.reconcile({
                rows,
                now,
                database: store.database,
                store,
                caller,
                signal: reconciliation.signal,
                ...(reconciliation.epoch === undefined ? {} : { epoch: reconciliation.epoch }),
                changed: () => reconciliation.changed(),
                mutate,
                observe,
            });

            // clear the failures the objects recorded before
            const failed = object.isControlled
                ? rows.filter((row) => !ObjectController.isSynced(row))
                : [];
            const reread = await ObjectController.reread(store, object, failed);
            await observeAll(reread, { conditions: { [SYNCED]: SYNCED_CONDITION } });

            return delay;
        } catch (error) {
            // record a declared failure no retry may resolve
            const failure = Failure.of(error);
            if (object.isControlled && Failure.isDeclared(error) && !Failure.isTransient(failure)) {
                const condition = ObjectController.failedCondition(failure);
                const observed = rows.map((row) => ({ ...row, ...ObservedFields.parse(row) }));
                await observeAll(observed, { conditions: { [SYNCED]: condition } });
            }
            throw error;
        }
    },

    /** Report whether a change only observed a controlled object, its generation unchanged. */
    isObserved(object: ObjectType, change: Change): boolean {
        return (
            object.isControlled &&
            change.operation === "update" &&
            change.before["generation"] === change.after["generation"]
        );
    },

    /** Report whether a controlled object's last reconcile succeeded or never failed. */
    isSynced(row: Row): boolean {
        return ObservedFields.parse(row).conditions[SYNCED]?.status !== "false";
    },

    /** Describe a failed reconcile as Crossplane's ReconcileError, findable by its reason. */
    failedCondition(failure: Failure): ObservationReport["conditions"][string] {
        return {
            status: "false",
            reason: "ReconcileError",
            message: `${failure.domain} ${failure.reason}: ${failure.message}`,
        };
    },

    /** Read controlled objects again after a reconcile, leaving out the removed ones. */
    async reread(
        store: ObjectStore,
        object: ObjectType,
        rows: readonly Row[],
    ): Promise<(ObservedRecord & Readonly<Record<string, unknown>>)[]> {
        // skip the read for no rows
        if (rows.length === 0) {
            return [];
        }

        // read the rows by identifier
        const ids = rows.map((row) => ObjectController.readIdentifier(row));
        const fresh = await Snapshot.live(store.database)
            .read(object.table)
            .findMany({ where: { id: { in: ids } } });

        return fresh.map((row) => ({ ...row, ...ObservedFields.parse(row) }));
    },
};
