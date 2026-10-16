import { ObjectReference } from "@destack/sync";
import { Duration, present, type schema } from "@destack/schema";
import { type Row, type Select } from "@destack/db";
import { CallError } from "@destack/runtime/error";
import type { MethodContext } from "./context.ts";
import { defineMethod, type Method } from "./method.ts";
import { MethodBuilder } from "./record.ts";
import { Step } from "./step.ts";
import { Empty, type MethodProcedure, type ObjectSchema } from "./procedure.ts";

import type { RecoverableDefinition } from "../trait/recoverable.ts";
import type { TraitTable } from "../object/table.ts";
import type { Execution } from "./execution.ts";
/** The method declarations of recoverable objects whose purge retains the record. */
const retainedMethod = new MethodBuilder<RetainedTable>();

/** A recoverable object type's table, with its deletion columns. */
type RecoverableTable = TraitTable<{ readonly recoverable: RecoverableDefinition }>;

/** A recoverable object type's table that retains purged records, with its purge column. */
type RetainedTable = TraitTable<{
    readonly recoverable: RecoverableDefinition & { readonly retain: "record" };
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
                handler: (context) => RecoverableMethod.restore(context),
                inverse: (step: Step) => {
                    // delete the restored object to its trash again
                    const call = Step.call(step, "delete", Step.target(step));

                    return call === undefined ? undefined : [call];
                },
            });
        }
        // purge a retained record by the type's own handler, which destroys its content
        else if (options.retain === "record") {
            return defineMethod<RestorationConfiguration & { table: RetainedTable }>({
                ...restorationShape(kind, permission),
                isPredicted: false,
                result: "value",
                ...(options.prepared === undefined ? {} : { prepared: options.prepared }),
                handler: (context) => RecoverableMethod.purgeRetained(context),
                async execute(this: Method, context: MethodContext, execution: Execution) {
                    // require a purge handler of the type's own
                    if (this.hooks?.handle === undefined) {
                        throw new TypeError(
                            `object ${context.object.name} retains purged records, so a purge handler of its own destroys their content`,
                        );
                    }

                    return this.handler(context, execution);
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
                handler: (context) => RecoverableMethod.purge(context),
            });
        }
    },

    /** Remove a purged record at the loaded revision, by holders of the purge permission. */
    discard<const Permission extends string>(permission: Permission) {
        return retainedMethod
            .mutation({ permission, isInternal: true, output: Empty })
            .handle(async (context) => {
                // require a purged record
                if (!RecoverableMethod.isPurged(context.target)) {
                    throw new CallError("CONFLICT", {
                        message: `${context.object.name} is not purged`,
                    });
                }

                // delete it at the loaded revision
                await context.remove();

                return {};
            });
    },

    /** Read the recovery of a call's served object type, which its host may set. */
    readRecovery(context: MethodContext): RecoverableDefinition {
        return present(
            context.object.lifecycle.recoverable,
            `the recovery of ${context.object.name}`,
        );
    },

    /** Clear a deletion that is still recoverable. */
    async restore(
        context: MethodContext<{ readonly table: RecoverableTable }, unknown>,
    ): Promise<Select<RecoverableTable>> {
        // require an unpurged deletion within its window
        const { object } = context;
        const target = context.target;
        const requested = target.deletedAt;
        if (requested === null) {
            throw new CallError("CONFLICT", { message: `${object.name} is not deleted` });
        } else if (RecoverableMethod.isPurged(target)) {
            throw new CallError("CONFLICT", { message: `${object.name} is purged` });
        } else if (
            context.now - requested >=
            Duration.milliseconds(RecoverableMethod.readRecovery(context).within)
        ) {
            throw new CallError("CONFLICT", {
                message: `${object.name} is past its recovery window`,
            });
        }

        return context.update({
            deletedAt: null,
            ...ObjectReference.columns("deletedBy", undefined),
        });
    },

    /** Purge an object in the trash, removing it at the loaded revision. */
    async purge(
        context: MethodContext<{ readonly table: RecoverableTable }, unknown>,
    ): Promise<Record<string, never>> {
        RecoverableMethod.requireTrashed(context);
        await context.remove();

        return {};
    },

    /** Purge a retained record in the trash, marking its content destroyed. */
    async purgeRetained(
        context: MethodContext<{ readonly table: RetainedTable }, unknown>,
    ): Promise<Record<string, never>> {
        RecoverableMethod.requireTrashed(context);
        await context.update({ purgedAt: context.now });

        return {};
    },

    /** Require an object in the trash, purged at most once. */
    requireTrashed(context: MethodContext<{ readonly table: RecoverableTable }, unknown>): void {
        const { object } = context;
        const target = context.target;
        if (target.deletedAt === null) {
            throw new CallError("CONFLICT", { message: `${object.name} is not deleted` });
        } else if (RecoverableMethod.isPurged(target)) {
            throw new CallError("CONFLICT", { message: `${object.name} is purged` });
        }
    },

    /** Decide whether a purge already destroyed a retained record's content. */
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
