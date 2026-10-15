import { schema } from "@destack/schema";
import { Snapshot, TABLE, Transaction, type DatabaseConnection, Key } from "@destack/db";
import type { RowChange } from "@destack/db";
import type { Authorization } from "../server/authorization.ts";
import type { ObjectStore } from "../server/store.ts";
import type { Call, MethodResult, MethodCall } from "./call.ts";
import { Method, type MethodConfiguration, type MethodDeclaration } from "./method.ts";

/** The scope and identifier of a removed object's row. */
const REMOVED = schema.looseObject({ scope: schema.string(), id: schema.string() });

/** Where a call executes: the store with its transaction and authorization, or a client. */
export type Execution =
    | {
          /** The store executing the call. */
          readonly kind: "store";
          /** The store, which runs further calls and keeps the event log. */
          readonly store: ObjectStore;
          /** The authorization of the call's caller. */
          readonly authorization: Authorization;
          /** The transaction of the call's writes, committed after its calls ran. */
          readonly transaction: Transaction;
          /** The call's reads: its transaction, or a snapshot at a requested position or branch. */
          readonly view: Transaction | Snapshot;
      }
    | {
          /** A client predicting the call. */
          readonly kind: "client";
          /** The client's replica the prediction reads and writes. */
          readonly database: DatabaseConnection;
      };

/** A store executing a call, with its transaction and the caller's authorization. */
export type StoreExecution = Extract<Execution, { readonly kind: "store" }>;

/** Where a call executes: its view and its writes. */
export const Execution = {
    /** Read the store executing a method only a store executes, refusing a client's prediction. */
    store(execution: Execution, call: Pick<Call, "object" | "name">): StoreExecution {
        if (execution.kind !== "store") {
            throw new TypeError(`${call.object.name}.${call.name} executes in its store`);
        }

        return execution;
    },

    /** Read the snapshot an execution's reads see: the store's view, or the client's replica. */
    snapshot(execution: Execution): Snapshot {
        // read the store's view as a snapshot, else the client's live replica
        if (execution.kind === "client") {
            return Snapshot.live(execution.database);
        }
        const { view } = execution;

        return view instanceof Transaction ? view.view() : view;
    },

    /** Write rows of any tables: through the store's transaction, or into the client's replica. */
    async write(
        execution: Execution,
        call: Pick<Call, "objects" | "now">,
        ...changes: readonly RowChange[]
    ): Promise<void> {
        // write through the transaction in a store, keeping the readers of the written types in step
        if (execution.kind === "store") {
            const { transaction, authorization } = execution;
            await transaction.write(...changes);
            for (const change of changes) {
                const type = call.objects.find((object) => object.table === change.table);
                type?.keepReaders(transaction, change, call.now);

                // delete a removed scope's access and a removed principal's standing
                if (type !== undefined && change.before !== null && change.after === null) {
                    const { scope, id } = REMOVED.parse(change.before);
                    const removed = type.reference(scope, id);
                    if (type.policy.definition.scope === true) {
                        await authorization.remove(removed);
                    }
                    if (type.represents !== undefined) {
                        await authorization.authorizer.cascade(transaction, removed);
                    }
                }
            }

            return;
        }

        // remove and put each row in the client's replica, refusing a change of no row
        for (const { table, before, after } of changes) {
            if (after !== null) {
                await execution.database.upsert(table, [after]);
            } else if (before !== null) {
                await execution.database.delete(table).where(Key.match(table, before));
            } else {
                throw new TypeError(
                    `a write to ${table[TABLE].name} has neither a row before nor after`,
                );
            }
        }
    },

    /** Replace a method's behaviour by object's own code, which reads where a call executes. */
    handle<Configuration extends MethodConfiguration>(
        method: MethodDeclaration<Configuration>,
        handler: (
            call: MethodCall<Configuration>,
            execution: Execution,
        ) => Promise<MethodResult<Configuration>>,
    ): MethodDeclaration<Configuration> {
        const { source: _source, ...unsourced } = method;

        return Method.declaration({
            ...unsourced,
            handler: (call: Call, execution: Execution) =>
                handler(Method.requirePrepared(unsourced, call), execution),
        });
    },
};
