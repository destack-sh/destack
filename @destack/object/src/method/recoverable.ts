import { ObjectReference } from "@destack/sync";
import { Duration, present, type schema } from "@destack/schema";
import { type Row, type Select } from "@destack/db";
import { CallError } from "@destack/runtime/error";
import type { Call } from "./call.ts";
import { defineMethod, type Method } from "./method.ts";
import { MethodBuilder } from "./record.ts";
import { Step } from "./step.ts";
import { Empty } from "./procedure.ts";

import type { RecoverableDefinition } from "../trait/recoverable.ts";
import type { TraitTable } from "../object/table.ts";
import type { Execution } from "./execution.ts";
/** The method declarations of recoverable objects whose purge keeps the record. */
const keptMethod = new MethodBuilder<KeptTable>();

/** A recoverable object type's table, with its deletion columns. */
type RecoverableTable = TraitTable<{ readonly recoverable: RecoverableDefinition }>;

/** A recoverable object type's table that keeps purged records, with its purge column. */
type KeptTable = TraitTable<{
    readonly recoverable: RecoverableDefinition & { readonly keep: "record" };
}>;

/** The recoverable methods' handlers: restoring, purging and discarding trashed objects. */
export const RecoverableMethod = {
    /** Restore a deleted object, or purge it for good. */
    restoration(
        kind: "restore" | "purge",
        permission: string,
        options: RecoverableDefinition,
    ): Method<{
        kind: "restore" | "purge";
        permission: string;
        prepared: schema.Schema;
        mutates: true;
    }> {
        const isRestore = kind === "restore";

        return defineMethod<{
            kind: "restore" | "purge";
            permission: string;
            prepared: schema.Schema;
            mutates: true;
        }>({
            kind,
            permission,
            mutates: true,
            isPredicted: isRestore || options.keep !== "record",
            target: "object",
            result: isRestore ? "object" : "value",
            ...(isRestore || options.prepared === undefined ? {} : { prepared: options.prepared }),
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: `/{id}/${kind}` },
                input: shapes.target.extend(shapes.replay),
                output: isRestore ? shapes.row : Empty,
            }),
            handler: isRestore
                ? (call) => RecoverableMethod.restore(call.requireTrashable())
                : options.keep === "record"
                  ? RecoverableMethod.purging
                  : (call) => RecoverableMethod.purge(call.requireTrashable()),
            ...(isRestore
                ? {
                      inverse: (step: Step) => {
                          // delete the restored object to its trash again
                          const call = Step.call(step, "delete", Step.target(step));

                          return call === undefined ? undefined : [call];
                      },
                  }
                : {
                      async execute(this: Method, call: Call, execution: Execution) {
                          // require a purge handler for kept records
                          if (
                              RecoverableMethod.readRecovery(call).keep === "record" &&
                              this.handler === RecoverableMethod.purging
                          ) {
                              throw new TypeError(
                                  `object ${call.object.name} keeps purged records, so a purge handler of its own destroys their content`,
                              );
                          }

                          return this.handler(call, execution);
                      },
                  }),
        });
    },

    /** Remove a purged record at the loaded revision, by holders of the purge permission. */
    discard<const Permission extends string>(permission: Permission) {
        return keptMethod
            .mutation({ permission, isInternal: true, output: Empty })
            .handle(async (call) => {
                // require a purged record
                if (!RecoverableMethod.isPurged(call.target)) {
                    throw new CallError("CONFLICT", {
                        message: `${call.object.name} is not purged`,
                    });
                }

                // delete it at the loaded revision
                await call.remove();

                return {};
            });
    },

    /** The default purge of kept records, marking a type without a purge handler of its own. */
    purging: (call: Call) => RecoverableMethod.purgeKept(call.requireKept()),

    /** Read the recovery a call's served object type keeps, which its host may set. */
    readRecovery(call: Call): RecoverableDefinition {
        return present(call.object.lifecycle.recoverable, `the recovery of ${call.object.name}`);
    },

    /** Clear a deletion that is still recoverable. */
    async restore(
        call: Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>>,
    ): Promise<Select<TraitTable<{ readonly recoverable: RecoverableDefinition }>>> {
        // require an unpurged deletion within its window
        const { object } = call;
        const target = call.requireTarget();
        const requested = target.deletedAt;
        if (requested === null) {
            throw new CallError("CONFLICT", { message: `${object.name} is not deleted` });
        } else if (RecoverableMethod.isPurged(target)) {
            throw new CallError("CONFLICT", { message: `${object.name} is purged` });
        } else if (
            call.now - requested >=
            Duration.milliseconds(RecoverableMethod.readRecovery(call).within)
        ) {
            throw new CallError("CONFLICT", {
                message: `${object.name} is past its recovery window`,
            });
        }

        return call.update({ deletedAt: null, ...ObjectReference.columns("deletedBy", undefined) });
    },

    /** Purge an object in the trash, removing it at the loaded revision. */
    async purge(call: Call<RecoverableTable>): Promise<Record<string, never>> {
        RecoverableMethod.requireTrashed(call);
        await call.remove();

        return {};
    },

    /** Purge a kept record in the trash, marking its content destroyed. */
    async purgeKept(call: Call<KeptTable>): Promise<Record<string, never>> {
        RecoverableMethod.requireTrashed(call);
        await call.update({ purgedAt: call.now });

        return {};
    },

    /** Require an object in the trash, purged at most once. */
    requireTrashed(call: Call<RecoverableTable>): void {
        const { object } = call;
        const target = call.requireTarget();
        if (target.deletedAt === null) {
            throw new CallError("CONFLICT", { message: `${object.name} is not deleted` });
        } else if (RecoverableMethod.isPurged(target)) {
            throw new CallError("CONFLICT", { message: `${object.name} is purged` });
        }
    },

    /** Decide whether a purge already destroyed a kept record's content. */
    isPurged(row: Row): boolean {
        return row["purgedAt"] !== undefined && row["purgedAt"] !== null;
    },
};
