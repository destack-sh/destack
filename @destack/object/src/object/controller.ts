import type { Call } from "@destack/sync";
import {
    type Condition,
    type Controller,
    type DatabaseConnection,
    type Projection,
    eq,
    type KeyFields,
    type Reconciliation,
    type Relation,
    type RowImage,
    type Row,
    Snapshot,
    TABLE,
    type Table,
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

/** The scope fields whose change releases a scope's rows: its fence and its suspension. */
const RELEASING_FIELDS = ["movedTo", "suspendedAt"] as const;

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
    /** The tables whose changed rows wake the keys they reference. */
    readonly watches?: readonly ObjectWatch[];
    /** The keys worked on at once, one by default. */
    readonly concurrency?: number;
} & ObjectWork<Object>;

/** Another table whose changed rows wake the keys its relations reach, by name or as a relation. */
export type ObjectWatch = Omit<Watch, "path"> & {
    /** The relations from a changed row to the rows holding the keys, none for the row itself. */
    readonly path?: readonly (string | Relation)[];
};

/** Objects a type projects from input tables, projected again as the inputs' changes touch them. */
export interface ObjectProjection<Object extends ObjectType = ObjectType> {
    /** The projection's name within its type, absent for the type's own projection. */
    readonly name?: string;
    /** The type's field holding the key, its key field by default. */
    readonly key?: string;
    /** The inputs whose changed rows touch keys, beside the type's own rows. */
    readonly watches: readonly ObjectWatch[];
    /** Every key: one input's column, the type's own key column by default. */
    readonly all?: Projection["all"];
    /** The controlled objects each pass reports Synced on, the projected objects by default. */
    readonly reports?: ObjectReport<Object>;
    /** Project the object of one key again from its inputs as they are now, absent or not. */
    project(key: string, reconciliation: ObjectReconciliation<Object>): Promise<void>;
}

/** The controlled objects a projection reports Synced on, as Crossplane's managed resource. */
export interface ObjectReport<Object extends ObjectType = ObjectType> {
    /** The controlled type reported on. */
    readonly object: ObjectType;
    /** Read the identifiers of the objects reported on for a key's projected objects. */
    identifiers(rows: readonly RowImage<Object["table"]>[]): readonly string[];
}

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
        // list the type again once a scope's fence or suspension changes, which releases its rows
        const key = declared.key ?? ["id"];
        const shape = {
            name: declared.name === undefined ? object.name : `${object.name}:${declared.name}`,
            table: object.table,
            pending: declared.pending,
            key,
            watches: [
                ...(declared.watches ?? []).map((watch) => ObjectController.watch(store, watch)),
                ObjectController.released(),
            ],
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

    /** Build a type's projection over its own rows and its inputs, keyed by its key field. */
    project(store: ObjectStore, object: ObjectType, declared: ObjectProjection): Projection {
        // key the projection by its field, listing every key from its input
        const field = declared.key ?? object.keyed ?? "id";
        const keyColumn = object.table[TABLE].column(field);
        const all = declared.all ?? { table: object.table, column: field };

        // watch the type's own changed rows by the field, unless a declared watch reads them
        const isOwnWatched = declared.watches.some((watch) => watch.table === object.table);
        const own: Watch = { table: object.table, on: { [all.column]: field } };
        const name = declared.name === undefined ? object.name : `${object.name}:${declared.name}`;

        return {
            name: `project:${name}`,
            key: field,
            watches: [
                ...(isOwnWatched ? [] : [own]),
                ...declared.watches.map((watch) => ObjectController.watch(store, watch)),
                ObjectController.released(),
            ],
            all,
            project: async (key, reconciliation) => {
                // project an absent object or one of the store's scopes, left to its home where it is a replica
                const rows = await store.database
                    .select()
                    .from(object.table)
                    .where(eq(keyColumn, key));
                const originated = await ObjectController.originated(store, object, rows);
                if (rows.length === 0 || originated.length > 0) {
                    const context = ObjectController.context(
                        store,
                        object,
                        originated,
                        reconciliation,
                    );
                    await ObjectController.projectKey(store, object, declared, key, context);
                }
            },
        };
    },

    /** Watch the scopes whose fence or suspension changes, listing the job again. */
    released(): Watch {
        return { table: Scope.table, fields: [...RELEASING_FIELDS], isListing: true };
    },

    /** Resolve a watch's relation names along its path through the store's relations. */
    watch(store: ObjectStore, declared: ObjectWatch): Watch {
        // leave a watch without a path as it is
        const { path, ...rest } = declared;
        if (path === undefined) {
            return rest;
        }

        // follow each named relation from the table the path reached
        let table: Table = declared.table;
        const relations = path.map((step) => {
            const relation =
                typeof step === "string" ? store.relations.relations.get(table, step) : step;
            table = relation.table;

            return relation;
        });

        return { ...rest, path: relations };
    },

    /** Select the rows of active scopes the store writes now, fenced ones only for a mover. */
    async originated(
        store: ObjectStore,
        object: ObjectType,
        rows: readonly Row[],
    ): Promise<readonly Row[]> {
        // read each scope's chain once, and decide whether the store owns it
        const snapshot = Snapshot.live(store.database);
        const isMoving = (object.policy.definition.moving ?? []).length > 0;
        const scopes = [...new Set(rows.map((row) => schema.string().parse(row["scope"])))];
        const chains = await Scope.chains(snapshot, scopes);
        const owned = new Set(
            (
                await Promise.all(
                    scopes.map(async (scope) => {
                        // own a scope that is placed here, unfenced, active, and no replica
                        const links = found(chains, scope);
                        const [link] = links;
                        const isFenced =
                            link === undefined || (link.movedTo !== undefined && !isMoving);
                        const isSuspended = links.some((each) => each.isSuspended);
                        const isReplica = await store.isReplica(object, snapshot, scope);

                        return isFenced || isSuspended || isReplica ? [] : [scope];
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
        // build the context and the observation of the rows the pass read
        const context = ObjectController.context(store, object, rows, reconciliation);
        const { mutate } = context;
        const isObserved = ObjectController.isObserved(store, object);
        const observeRead = async (observation: ObservationReport) => {
            // observe each row at the generation the pass read, leaving out the ones removed since
            const read = (isObserved ? rows : []).map((row) => ({
                ...row,
                ...ObservedFields.parse(row),
            }));
            const reread = await ObjectController.reread(
                store,
                object,
                read.map((row) => row.id),
            );
            const kept = new Set(reread.map((row) => row.id));
            const calls = read
                .filter((row) => kept.has(row.id) && !Observation.isRecorded(row, observation))
                .map((row) => Observation.call(object, row, observation));
            if (calls.length > 0) {
                await mutate(calls);
            }
        };

        // reconcile, recording a lasting failure on each controlled object until its next change
        try {
            // reconcile, then record each converged object synced at the generation the pass read
            const outcome = await declared.reconcile(context);
            if (outcome !== "waiting") {
                await observeRead({ conditions: { [SYNCED]: SYNCED_CONDITION } });
            }
        } catch (error) {
            // hold a pass a suspension refused midway, until the resume selects it again
            const failure = Failure.of(error);
            if (failure.reason === "SCOPE_DENIED") {
                return;
            }

            // record a declared failure no retry may resolve
            if (Failure.isDeclared(error) && !Failure.isTransient(failure)) {
                const condition = ObjectController.failedCondition(failure);
                await observeRead({ conditions: { [SYNCED]: condition } });
            }
            throw error;
        }
    },

    /** Report whether a store records conditions on a controlled type it does not replicate. */
    isObserved(store: ObjectStore, object: ObjectType): boolean {
        return object.isControlled && !store.database.replicates(object.table);
    },

    /** Project one key, recording Synced on the objects it reports on once projected or failed. */
    async projectKey(
        store: ObjectStore,
        object: ObjectType,
        declared: ObjectProjection,
        key: string,
        context: ObjectReconciliation,
    ): Promise<void> {
        // record a condition on each reported object of a controlled type that lacks it
        const reported = declared.reports ?? {
            object,
            identifiers: (rows) => rows.map((row) => ObjectController.readIdentifier(row)),
        };
        const record = async (condition: ObservationReport["conditions"][string]) => {
            // read the objects again as the pass left them, none the store only replicates
            const rows = ObjectController.isObserved(store, reported.object)
                ? await ObjectController.reread(
                      store,
                      reported.object,
                      reported.identifiers(context.rows),
                  )
                : [];

            // observe each one lacking the condition, in one mutation
            const observation = { conditions: { [SYNCED]: condition } };
            const calls = rows
                .filter((row) => !Observation.isRecorded(row, observation))
                .map((row) => Observation.call(reported.object, row, observation));
            if (calls.length > 0) {
                await context.mutate(calls);
            }
        };

        // project, recording the objects synced, or failed for a declared failure no retry resolves
        try {
            await declared.project(key, context);
            await record(SYNCED_CONDITION);
        } catch (error) {
            // hold a pass a suspension refused midway, until the resume projects it again
            const failure = Failure.of(error);
            if (failure.reason === "SCOPE_DENIED") {
                return;
            }

            // record a lasting failure, then report it
            if (Failure.isDeclared(error) && !Failure.isTransient(failure)) {
                await record(ObjectController.failedCondition(failure));
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

    /** Read controlled objects again by identifier after a pass, leaving out the removed ones. */
    async reread(
        store: ObjectStore,
        object: ObjectType,
        ids: readonly string[],
    ): Promise<(ObservedRecord & Readonly<Record<string, unknown>>)[]> {
        // skip the read for no objects
        if (ids.length === 0) {
            return [];
        }

        // read the objects by identifier
        const fresh = await Snapshot.live(store.database)
            .read(object.table)
            .findMany({ where: { id: { in: ids } } });

        return fresh.map((row) => ({ ...row, ...ObservedFields.parse(row) }));
    },
};
