import { type Identifier, schema } from "@destack/schema";
import { Snapshot, Transaction } from "@destack/db";
import type { Row, RowChange } from "@destack/db";
import type { Manager, Permission } from "@destack/access";
import type { ObjectType } from "../object/object.ts";
import type { Authorization } from "../server/authorization.ts";
import type { ObjectStore } from "../server/store.ts";
import type { MethodResult, MethodContext } from "./context.ts";
import { Method, type MethodConfiguration, type MethodDeclaration } from "./method.ts";

/** The scope and identifier of a removed object's row. */
const REMOVED = schema.looseObject({ scope: schema.string(), id: schema.string() });

/** Where a call executes: a store's transaction at the home, or predicting the home. */
export type Execution = {
    /** The store executing the call. */
    readonly kind: "store";
    /** The store, which runs further calls and writes the event log. */
    readonly store: ObjectStore;
    /** The transaction of the call's writes, committed or recorded as a prediction. */
    readonly transaction: Transaction;
    /** The call's reads: its transaction, or a snapshot at a requested position or branch. */
    readonly view: Transaction | Snapshot;
    /** The identifier the call's record takes. */
    readonly callId: Identifier<"call">;
    /** The call that ran this one inside its transaction, absent for a sent call. */
    readonly parentId?: Identifier<"call">;
} & (
    | {
          /** The home executes the call. */
          readonly isPredicted: false;
          /** The authorization of the call's caller. */
          readonly authorization: Authorization;
      }
    | {
          /** The store predicts its home's execution, whose authorization decides the call. */
          readonly isPredicted: true;
      }
);

/** A store executing a call at the home, with the caller's authorization. */
export type StoreExecution = Extract<Execution, { readonly isPredicted: false }>;

/** Where a call executes: its view and its writes. */
export const Execution = {
    /** Read the store executing a method only its home executes, refusing a prediction. */
    store(execution: Execution, context: Pick<MethodContext, "object" | "name">): StoreExecution {
        if (execution.isPredicted) {
            throw new TypeError(
                `'${context.object.name}.${context.name}' can only execute at its home`,
            );
        }

        return execution;
    },

    /** Read the snapshot an execution's reads see: its transaction's, or the requested view. */
    snapshot(execution: Execution): Snapshot {
        const { view } = execution;

        return view instanceof Transaction ? view.view() : view;
    },

    /** Write rows through the store's transaction, clearing removed access at the home. */
    async write(
        execution: Execution,
        context: Pick<MethodContext, "objects" | "now">,
        ...changes: readonly RowChange[]
    ): Promise<void> {
        // write through the transaction, leaving removed access to the home a prediction's changes reach
        const { transaction } = execution;
        await transaction.write(...changes);
        if (execution.isPredicted) {
            return;
        }
        const { authorization } = execution;
        for (const change of changes) {
            const type = context.objects.find((object) => object.table === change.table);

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
    },

    /** Keep the rows the caller holds a permission on, every row in a prediction. */
    async permitted(
        execution: Execution,
        object: ObjectType,
        permission: Permission,
        rows: readonly Row[],
    ): Promise<readonly Row[]> {
        return execution.isPredicted
            ? rows
            : (await execution.authorization.filter(object, permission, rows)).rows;
    },

    /** Read the manager the call writes for at the home, none in a prediction. */
    manager(execution: Execution): Manager | null {
        return execution.isPredicted ? null : execution.authorization.manager;
    },

    /** Replace a method's behaviour by object's code, which reads where a call executes. */
    handle<Configuration extends MethodConfiguration>(
        method: MethodDeclaration<Configuration>,
        handler: (
            context: MethodContext<Configuration>,
            execution: Execution,
        ) => Promise<MethodResult<Configuration>>,
    ): MethodDeclaration<Configuration> {
        const { source: _source, ...unsourced } = method;

        return Method.declaration({
            ...unsourced,
            handler: (context: MethodContext, execution: Execution) =>
                handler(Method.requirePrepared(unsourced, context), execution),
        });
    },
};
