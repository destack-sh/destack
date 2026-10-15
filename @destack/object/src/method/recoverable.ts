import { ObjectReference } from "@destack/sync";
import { Duration, type JsonObject, present, type schema } from "@destack/schema";
import { type Row, type Select } from "@destack/db";
import { CallError } from "@destack/runtime/error";
import type { Call } from "./call.ts";
import { defineMethod, type Method } from "./method.ts";
import { MethodBuilder } from "./record.ts";
import { Step } from "./step.ts";
import { Empty, type MethodProcedure, type ObjectSchema } from "./procedure.ts";

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

/** The signature of a restoration: a restore or a purge, with the purge's prepared work. */
type RestorationConfiguration = {
    kind: "restore" | "purge";
    permission: string;
    prepared: schema.Schema;
    mutates: true;
};

/** The recoverable methods' handlers: restoring, purging and discarding trashed objects. */
export const RecoverableMethod = {
    /** Restore a deleted object, or purge it for good. */
    restoration(
        kind: "restore" | "purge",
        permission: string,
        options: RecoverableDefinition,
    ): Method<RestorationConfiguration> {
        // restore within the recovery window, undone by deleting again
        if (kind === "restore") {
            return defineMethod<RestorationConfiguration & { table: RecoverableTable }>({
                ...restorationShape(kind, permission),
                isPredicted: true,
                result: "object",
                handler: (call) => RecoverableMethod.restore(call),
                inverse: (step: Step) => {
                    // delete the restored object to its trash again
                    const call = Step.call(step, "delete", Step.target(step));

                    return call === undefined ? undefined : [call];
                },
            });
        }
        // purge a kept record by the type's own handler, which destroys its content
        else if (options.keep === "record") {
            return defineMethod<RestorationConfiguration & { table: KeptTable }>({
                ...restorationShape(kind, permission),
                isPredicted: false,
                result: "value",
                ...(options.prepared === undefined ? {} : { prepared: options.prepared }),
                handler: (call) => RecoverableMethod.purgeKept(call),
                async execute(this: Method, call: Call, execution: Execution) {
                    // require a purge handler of the type's own
                    if (this.hooks?.handle === undefined) {
                        throw new TypeError(
                            `object ${call.object.name} keeps purged records, so a purge handler of its own destroys their content`,
                        );
                    }

                    return this.handler(call, execution);
                },
            });
        }
        // purge any other object, removing it
        else {
            return defineMethod<RestorationConfiguration & { table: RecoverableTable }>({
                ...restorationShape(kind, permission),
                isPredicted: true,
                result: "value",
                ...(options.prepared === undefined ? {} : { prepared: options.prepared }),
                handler: (call) => RecoverableMethod.purge(call),
            });
        }
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

    /** Read the recovery a call's served object type keeps, which its host may set. */
    readRecovery(call: Call): RecoverableDefinition {
        return present(call.object.lifecycle.recoverable, `the recovery of ${call.object.name}`);
    },

    /** Clear a deletion that is still recoverable. */
    async restore(
        call: Call<RecoverableTable, JsonObject, Select<RecoverableTable>>,
    ): Promise<Select<RecoverableTable>> {
        // require an unpurged deletion within its window
        const { object } = call;
        const target = call.target;
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
    async purge(
        call: Call<RecoverableTable, JsonObject, Select<RecoverableTable>>,
    ): Promise<Record<string, never>> {
        RecoverableMethod.requireTrashed(call);
        await call.remove();

        return {};
    },

    /** Purge a kept record in the trash, marking its content destroyed. */
    async purgeKept(
        call: Call<KeptTable, JsonObject, Select<KeptTable>>,
    ): Promise<Record<string, never>> {
        RecoverableMethod.requireTrashed(call);
        await call.update({ purgedAt: call.now });

        return {};
    },

    /** Require an object in the trash, purged at most once. */
    requireTrashed(call: Call<RecoverableTable, JsonObject, Select<RecoverableTable>>): void {
        const { object } = call;
        const target = call.target;
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

/** Declare what every restoration shares: its kind, permission, target and route. */
function restorationShape(kind: "restore" | "purge", permission: string) {
    return {
        kind,
        permission,
        mutates: true,
        target: "object",
        procedure: (_name: string, shapes: ObjectSchema): MethodProcedure => ({
            route: { method: "POST", path: `/{id}/${kind}` },
            input: shapes.target.extend(shapes.replay),
            output: kind === "restore" ? shapes.row : Empty,
        }),
    } as const;
}
