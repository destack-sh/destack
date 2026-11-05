import type { Call } from "@destack/sync";
import {
    Change,
    type Condition,
    type Cursor,
    type Controller,
    type DatabaseConnection,
    Projection,
    eq,
    type KeyFields,
    type Reconciliation,
    type RowImage,
    type Row,
    Scalar,
    Snapshot,
    type Table,
    TABLE,
    type Watch,
} from "@destack/db";
import { AccessContext } from "@destack/access";
import { RequestId, Scope } from "@destack/sync";
import { Failure } from "@destack/error";
import { found, schema } from "@destack/schema";
import { waitAborted } from "@destack/environment";
import type { ObjectType } from "./object.ts";
import {
    ConditionMap,
    SYNCED,
    Observation,
    type ObservationReport,
    type ObservedRecord,
} from "./condition.ts";
import { type ResultsOf } from "../method/context.ts";
import type { MutateOptions, ObjectStore } from "../server/store.ts";
import { CallSite } from "../method/site.ts";

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

/** The scalar fields of a key, as watches select them. */
const KEY_FIELDS = schema.record(schema.string(), Scalar);

/** The fields grouping objects a controller reconciles together, in JSON form. */
export type { KeyFields };

/** What a type's controller does with its pending objects: converge them, or hold work. */
type ObjectWork<Object extends ObjectType> =
    | {
          /** Converge one key's pending objects, or report them waiting for a change. */
          reconcile(reconciliation: ObjectReconciliation<Object>): Promise<void | "waiting">;
      }
    | {
          /** Hold one key's work in the process while its objects stay pending, until stopped. */
          follow(following: ObjectFollowing<Object>): Promise<void>;
      };

/** A type's reactive behavior beside its methods: the objects with work, and the work. */
export type ObjectController<Object extends ObjectType = ObjectType> = {
    /** The controller's name within its type, absent for the type's own controller. */
    readonly name?: string;
    /** The objects with work now, which may compare time fields to `now`. */
    readonly pending: Condition<RowImage<Object["table"]>>;
    /** The fields grouping objects reconciled together, the identifier by default. */
    readonly key?: readonly (keyof RowImage<Object["table"]> & string)[];
    /** The tables whose changed rows select keys beside the type's own pending objects. */
    readonly watches?: readonly ObjectWatch[];
    /** The keys worked on at once, one by default. */
    readonly concurrency?: number;
} & ObjectWork<Object>;

/** Objects a type projects from input tables, projected again as the inputs' changes touch them. */
export interface ObjectProjection<Object extends ObjectType = ObjectType> {
    /** The projection's name within its type, absent for the type's own projection. */
    readonly name?: string;
    /** The input tables, the type's own among them, and the keys their changed rows touch. */
    readonly watches: readonly ObjectWatch[];
    /** Project the object of one key again from its inputs as they are now, absent or not. */
    project(key: string, reconciliation: ObjectReconciliation<Object>): Promise<void>;
}

/** Another table whose changed rows select a controller's keys. */
export interface ObjectWatch<Watched extends Table = Table> {
    /** The watched table. */
    readonly table: Watched;
    /** Whether observations of its rows select keys too, as an owner waits for what it owns. */
    readonly isObserving?: boolean;
    /** Read the keys a changed row selects, given the change with its row before and after. */
    keys(
        row: RowImage<Watched>,
        database: DatabaseConnection,
        change: Change<Watched>,
    ): readonly KeyFields[] | Promise<readonly KeyFields[]>;
}

/** Watch other tables for a controller. */
export const ObjectWatch = {
    /** Watch a table's changed rows, typing them by the table, observations too if asked. */
    of<Watched extends Table>(
        table: Watched,
        keys: ObjectWatch<Watched>["keys"],
        options: { readonly isObserving?: boolean } = {},
    ): ObjectWatch<Watched> {
        return { table, keys, ...options };
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

/** One key's follow: its pending objects, held while pending, reporting once caught up. */
export interface ObjectFollowing<
    Object extends ObjectType = ObjectType,
> extends ObjectReconciliation<Object> {
    /** Report the follow caught up with its source, once per follow, which settling waits for. */
    readonly synced: () => void;
}

/** Build the controllers object types declare. */
export const ObjectController = {
    /** Build a type's declared controller over its table, as the store's principal. */
    control(store: ObjectStore, object: ObjectType, declared: ObjectController): Controller {
        // select a watched table's keys from each changed row's latest image
        const key = declared.key ?? ["id"];
        const watches = (declared.watches ?? []).map((watch): Watch => ({
            table: watch.table,
            ...(watch.isObserving === undefined ? {} : { isObserving: watch.isObserving }),
            keys: (change, database) => watch.keys(Change.image(change), database, change),
        }));

        // select a scope's keys again once its transfer fence lifts
        const unfenced: Watch = {
            table: Scope.table,
            keys: async (change, database) => {
                // pass every change but a lifted fence, read in memory
                const before = Change.before(change);
                const after = Change.after(change);
                if (
                    before === null ||
                    after === null ||
                    before["movedTo"] === null ||
                    after["movedTo"] !== null
                ) {
                    return [];
                }

                // select the scope's keys
                const scope = schema.string().parse(after["scope"]);
                const columns = Object.fromEntries(
                    key.map((field) => [field, object.table[TABLE].column(field)]),
                );
                const rows = await database
                    .selectDistinct(columns)
                    .from(object.table)
                    .where(object.inScope(scope));

                return rows.map((row) => KEY_FIELDS.parse(row));
            },
        };
        const shape = {
            name: declared.name === undefined ? object.name : `${object.name}:${declared.name}`,
            table: object.table,
            pending: declared.pending,
            key,
            watches: [...watches, unfenced],
            ...(declared.concurrency === undefined ? {} : { concurrency: declared.concurrency }),
        };

        // hold a follow's work while its objects stay pending, else converge them
        return "follow" in declared
            ? {
                  ...shape,
                  follow: async (rows, following) => {
                      // hold no work for the rows of replicated or fenced scopes, which their home follows
                      const originated = await ObjectController.originated(store, object, rows);
                      if (originated.length === 0) {
                          following.synced();
                          await waitAborted(following.signal);

                          return;
                      }

                      // follow the store's own rows
                      const context = ObjectController.context(
                          store,
                          object,
                          originated,
                          following,
                      );
                      await declared.follow({ ...context, synced: following.synced });
                  },
              }
            : {
                  ...shape,
                  reconcile: async (rows, reconciliation) => {
                      // leave the rows of replicated scopes to their home, which writes them
                      const originated = await ObjectController.originated(store, object, rows);
                      if (originated.length > 0) {
                          await ObjectController.reconcileRows(
                              store,
                              object,
                              declared,
                              originated,
                              reconciliation,
                          );
                      }
                  },
              };
    },

    /** Build a type's projection as a consumer of its inputs' changes past one checkpoint. */
    project(store: ObjectStore, object: ObjectType, declared: ObjectProjection): Cursor {
        // read the touched keys: the watches' of the changed table, else the type's own changed row's
        const field = object.keyed ?? "id";
        const watches = Map.groupBy(declared.watches, (watch) => watch.table);
        const keys = async (change: Change, database: DatabaseConnection) => {
            const watching = watches.get(change.table);
            const touched =
                watching !== undefined
                    ? (
                          await Promise.all(
                              watching.map(async (watch) =>
                                  watch.keys(Change.image(change), database, change),
                              ),
                          )
                      ).flat()
                    : change.table === object.table
                      ? [Change.image(change)]
                      : [];

            return [...new Set(touched.map((fields) => schema.string().parse(fields[field])))];
        };
        const name = declared.name === undefined ? object.name : `${object.name}:${declared.name}`;
        const keyColumn = object.table[TABLE].column(field);

        return new Projection(store.database, {
            name: `project:${name}`,
            tables: [...new Set([object.table, ...watches.keys()])],
            observed: declared.watches.flatMap((watch) =>
                watch.isObserving === true ? [watch.table] : [],
            ),
            keys,
            all: async (database) =>
                (await database.select({ key: keyColumn }).from(object.table)).map((row) =>
                    schema.string().parse(row.key),
                ),
            project: async (key, reconciliation) => {
                // project an absent object or one of the store's scopes, left to its home where it is a replica
                const rows = await store.database
                    .select()
                    .from(object.table)
                    .where(eq(keyColumn, key));
                const originated = await ObjectController.originated(store, object, rows);
                if (rows.length === 0 || originated.length > 0) {
                    await declared.project(
                        key,
                        ObjectController.context(store, object, originated, reconciliation),
                    );
                }
            },
        });
    },

    /** Select the rows of scopes the store writes now: no replicas, and fenced ones for a mover. */
    async originated(
        store: ObjectStore,
        object: ObjectType,
        rows: readonly Row[],
    ): Promise<readonly Row[]> {
        // read each scope's chain once, and decide whether its rows are replicas, a type moving scopes working in fenced ones
        const snapshot = Snapshot.live(store.database);
        const isMoving = (object.policy.definition.moving ?? []).length > 0;
        const scopes = [...new Set(rows.map((row) => schema.string().parse(row["scope"])))];
        const chains = await Scope.chains(snapshot, scopes);
        const owned = new Set(
            (
                await Promise.all(
                    scopes.map(async (scope) => {
                        // own a scope that is placed here, unfenced, and no replica
                        const [link] = found(chains, scope);
                        const isFenced =
                            link === undefined || (link.movedTo !== undefined && !isMoving);
                        const isReplica = await store.isReplica(object, snapshot, scope);

                        return isFenced || isReplica ? [] : [scope];
                    }),
                )
            ).flat(),
        );

        return rows.filter((row) => owned.has(schema.string().parse(row["scope"])));
    },

    /** Read an object row's identifier, which every object table has. */
    readIdentifier(row: Row): string {
        return schema.string().parse(row["id"]);
    },

    /** Build the context a key's reconciliation acts in, as the type's serving principal. */
    context(
        store: ObjectStore,
        object: ObjectType,
        rows: readonly Row[],
        reconciliation: Reconciliation,
    ): ObjectReconciliation {
        // reconcile as the type's serving principal and implementing package, at one time
        const now = store.clock.now();
        const caller = store.as(object, now);
        const site = CallSite.package(store.implementer(object), (type) => store.implementer(type));
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

        return {
            rows,
            now,
            database: store.database,
            store,
            caller,
            signal: reconciliation.signal,
            changed: () => reconciliation.changed(),
            mutate,
            observe,
        };
    },

    /** Reconcile one key's pending objects as the type's serving principal. */
    async reconcileRows(
        store: ObjectStore,
        object: ObjectType,
        declared: Extract<ObjectController, { readonly reconcile: unknown }>,
        rows: readonly Row[],
        reconciliation: Reconciliation,
    ): Promise<void> {
        // build the context, and observe a report on every row lacking it in one mutation, where the database writes the type's rows
        const context = ObjectController.context(store, object, rows, reconciliation);
        const { mutate } = context;
        const isObserved = object.isControlled && !store.database.replicates(object.table);
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
            const outcome = await declared.reconcile(context);

            // record each converged controlled object synced at the generation it has now
            const reread = await ObjectController.reread(
                store,
                object,
                isObserved && outcome !== "waiting" ? rows : [],
            );
            await observeAll(reread, { conditions: { [SYNCED]: SYNCED_CONDITION } });
        } catch (error) {
            // record a declared failure no retry may resolve
            const failure = Failure.of(error);
            if (isObserved && Failure.isDeclared(error) && !Failure.isTransient(failure)) {
                const condition = ObjectController.failedCondition(failure);
                const observed = rows.map((row) => ({ ...row, ...ObservedFields.parse(row) }));
                await observeAll(observed, { conditions: { [SYNCED]: condition } });
            }
            throw error;
        }
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
