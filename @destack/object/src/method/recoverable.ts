import { RecordMethod } from "./record.ts";
import { ObjectError } from "../error/error.ts";
import { ObjectReference } from "@destack/sync";
import { Duration, present, schema } from "@destack/schema";
import { type Row, type Select } from "@destack/db";

import type { MethodContext } from "./context.ts";
import { defineMethod, Method } from "./method.ts";
import { MethodBuilder } from "./record.ts";
import { Step } from "./step.ts";
import { Empty, type MethodProcedure, type ObjectSchema } from "./procedure.ts";

import type { RecoverableDefinition } from "../trait/recoverable.ts";
import type { TraitTable } from "../object/table.ts";
import type { Execution } from "./execution.ts";

import type {} from "@destack/package/import-meta";
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
        // purge a retained record by the type's handler, which destroys its content
        else if (options.retain === "record") {
            return defineMethod<RestorationConfiguration & { table: RetainedTable }>({
                ...restorationShape(kind, permission),
                isPredicted: false,
                result: "value",
                ...(options.prepared === undefined ? {} : { prepared: options.prepared }),
                handler: (context) => RecoverableMethod.purgeRetained(context),
                async execute(this: Method, context: MethodContext, execution: Execution) {
                    // require the type to implement the purge
                    if (this.hooks?.handle === undefined) {
                        throw new TypeError(
                            `object ${context.object.name} keeps purged records, so its purge must destroy their content`,
                        );
                    }

                    return Method.handle(this, context, execution);
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
                    throw ObjectError.of("PURGE_REQUIRED", `${context.object.name} is not purged`);
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
            throw ObjectError.of("DELETION_REQUIRED", `${object.name} is not deleted`);
        } else if (RecoverableMethod.isPurged(target)) {
            throw ObjectError.of("PURGED", `${object.name} is purged`);
        } else if (
            context.now - requested >=
            Duration.milliseconds(RecoverableMethod.readRecovery(context).within)
        ) {
            throw ObjectError.of("RECOVERY_EXPIRED", `${object.name} is past its recovery window`);
        }

        // restore it under a live parent, with the children trashed with it
        const row: Row = target;
        const parentId = row["parentId"];
        await RecordMethod.requireParentReady(
            context,
            parentId === undefined || parentId === null ? null : schema.string().parse(parentId),
        );
        const restored = await context.update({
            deletedAt: null,
            ...ObjectReference.columns("deletedBy", undefined),
        });
        await RecordMethod.trashChildren(context, schema.string().parse(row["id"]), "restore");

        return restored;
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
            throw ObjectError.of("DELETION_REQUIRED", `${object.name} is not deleted`);
        } else if (RecoverableMethod.isPurged(target)) {
            throw ObjectError.of("PURGED", `${object.name} is purged`);
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
        procedure: (_name: string, schemas: ObjectSchema): MethodProcedure => ({
            route: { method: "POST", path: `/{id}/${kind}` },
            input: schemas.target.extend(schemas.replay),
            output: kind === "restore" ? schemas.row : Empty,
        }),
    } as const;
}
