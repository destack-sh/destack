import { Duration, present, schema } from "@destack/schema";
import {
    type ColumnBuilder,
    identifier,
    integer,
    Now,
    type Row,
    type Select,
    text,
} from "@destack/db";
import { ObjectReference } from "@destack/sync";
import { t } from "@destack/locale";
import { ObjectError } from "../error/error.ts";
import type { MethodContext } from "../method/context.ts";
import type { Execution } from "../method/execution.ts";
import { defineMethod, Method } from "../method/method.ts";
import { method, MethodBuilder } from "../method/builder.ts";
import { Step } from "../method/step.ts";
import type { FieldColumn } from "../field/field.ts";
import type { TraitTable } from "../object/table.ts";
import { ObjectRecord } from "../method/record.ts";
import {
    Empty,
    type MethodProcedure,
    type ObjectSchema,
    type Procedure,
    type ReplayFields,
    type RowSchema,
    type TargetFields,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { TraitBehavior } from "./trait.ts";
import { callableTrait } from "./trait.ts";

/** The options of recoverable deletion. */
export type RecoverableDefinition<Permissions extends string = string> = {
    /** How long deleted objects stay restorable. */
    readonly within: Duration;
    /** The permission deleting, restoring and by default purging the objects. */
    readonly by: Permissions;
    /** The object permission whose holders purge the objects, `by` when absent. */
    readonly purge?: Permissions;
    /** What a purge retains, the record or nothing. */
    readonly retain?: "record";
    /** The external work a purge prepares before its transaction, such as releasing storage. */
    readonly prepared?: schema.Schema;
};

/** The columns recording the deletion and the purge of a retained record's content. */
export type DeletionBuilderMap<Deletion> = Deletion extends RecoverableDefinition
    ? DeletionColumns &
          (Deletion extends {
              readonly retain: "record";
          }
              ? { purgedAt: ColumnBuilder<FieldColumn<number, false, false>> }
              : {})
    : {};

/** The methods recoverable deletion derives. */
export type RecoverableMethodMap<Deletion> = Deletion extends RecoverableDefinition
    ? {
          readonly delete: Method<{ kind: "delete"; permission: Deletion["by"]; mutates: true }>;
          readonly restore: Method<{ kind: "restore"; permission: Deletion["by"]; mutates: true }>;
          readonly purge: Method<{
              kind: "purge";
              permission: PurgePermission<Deletion>;
              prepared: PurgePrepared<Deletion>;
              mutates: true;
          }>;
      } & (Deletion extends { readonly retain: "record" }
          ? {
                readonly discard: Method<{
                    kind: "custom";
                    permission: PurgePermission<Deletion>;
                    output: typeof Empty;
                    mutates: true;
                }>;
            }
          : {})
    : {};

/** The permission purging needs: the one `purge` sets, else `by`. */
type PurgePermission<Deletion extends RecoverableDefinition> = Deletion extends {
    readonly purge: infer Purge extends string;
}
    ? Purge
    : Deletion["by"];

/** The external work a purge prepares: the schema `prepared` sets, else none. */
type PurgePrepared<Deletion extends RecoverableDefinition> = Deletion extends {
    readonly prepared: infer Prepared extends schema.Schema;
}
    ? Prepared
    : never;

/** The most deletions one purge mutation removes: about a millisecond of deletes. */
const PURGE_ROWS = 100;

/** Objects deleted to a trash, restorable within a window and purged after it. */
const recoverableTrait: TraitBehavior<RecoverableDefinition> = {
    key: "recoverable",
    storages: ["durable"],
    options: (definition) => definition.recoverable,
    columns: (options) => ({
        ...Recoverable.columns(),
        ...(options.retain === "record"
            ? {
                  /** The time a purge destroyed the retained record's content, null before. */
                  purgedAt: integer("purged_at"),
              }
            : {}),
    }),
    constraints: () => [],
    methods: (options) => ({
        delete: method.delete(options.by),
        restore: Recoverable.restoration("restore", options.by, options),
        purge: Recoverable.restoration("purge", options.purge ?? options.by, options),
        ...(options.retain === "record"
            ? { discard: Recoverable.discard(options.purge ?? options.by) }
            : {}),
    }),
    // purge each scope's deletions as their windows end, a batch per mutation
    controllers: (options, object) => {
        // find the method purging the type's objects, whatever its name
        const [purge] = present(
            Object.entries(object.methods).find(([, declared]) => declared.kind === "purge"),
            `the purge method of ${object.name}`,
        );

        return [
            {
                name: "purge",
                when: {
                    deletedAt: { lte: new Now().minus(options.within) },
                    ...(options.retain === "record" ? { purgedAt: { isNull: true } } : {}),
                },
                by: ["scope"],
                call: purge,
                batch: PURGE_ROWS,
            },
        ];
    },
    validate: (options, object, definition) => {
        // refuse controlled objects
        if (definition.controlled !== undefined) {
            throw new TypeError(
                `object ${object.name} cannot be recoverable: its controller finishes its deletion`,
            );
        }

        // validate the window and retention
        Duration.require(options.within, `recovery window of ${object.name}`);
        if (options.retain !== undefined && options.retain !== "record") {
            throw new TypeError(`purges of ${object.name} must retain the record or nothing`);
        }
    },
};

/** Delete objects to a trash, restorable within a window. */
export const recoverable = callableTrait("recoverable", recoverableTrait);

/** The procedures recoverable deletion derives besides deleting. */
export type RecoverableProcedures<Object extends ObjectType> = {
    restore: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object>>,
        RowSchema<Object>
    >;
    purge: Procedure<schema.Object<TargetFields<Object> & ReplayFields<Object>>, schema.Object<{}>>;
};

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

/** The columns, methods and handlers of objects deleted to a trash. */
export const Recoverable = {
    /** Declare the time a record was deleted and who deleted it. */
    columns: deletionColumns,

    /** Restore a deleted object, or purge it for good. */
    restoration(
        kind: "restore" | "purge",
        permission: string,
        options: RecoverableDefinition,
    ): Method<RestorationConfiguration> {
        // restore within the recovery window, undone by deleting again
        if (kind === "restore") {
            return defineMethod<RestorationConfiguration & { table: RecoverableTable }>({
                title: t`Restore`,
                ...restorationShape(kind, permission),
                isPredicted: true,
                result: "object",
                handler: (context) => Recoverable.restore(context),
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
                title: t`Purge`,
                ...restorationShape(kind, permission),
                isPredicted: false,
                result: "value",
                ...(options.prepared === undefined ? {} : { prepared: options.prepared }),
                handler: (context) => Recoverable.purgeRetained(context),
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
                title: t`Restoration`,
                ...restorationShape(kind, permission),
                isPredicted: true,
                result: "value",
                ...(options.prepared === undefined ? {} : { prepared: options.prepared }),
                handler: (context) => Recoverable.purge(context),
            });
        }
    },

    /** Remove a purged record at the loaded revision, by holders of the purge permission. */
    discard<const Permission extends string>(permission: Permission) {
        return retainedMethod
            .mutation({ title: t`Discard`, permission, visibility: "package", output: Empty })
            .handle(async (context) => {
                // require a purged record
                if (!Recoverable.isPurged(context.target)) {
                    throw ObjectError.of(
                        "PURGE_BLOCKED",
                        `cannot discard ${context.object.name}: it is not purged`,
                    );
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
            throw ObjectError.of(
                "DELETION_BLOCKED",
                `cannot restore ${object.name}: it is not deleted`,
            );
        } else if (Recoverable.isPurged(target)) {
            throw ObjectError.of("PURGE_BLOCKED", `cannot restore ${object.name}: it is purged`);
        } else if (
            context.now - requested >=
            Duration.milliseconds(Recoverable.readRecovery(context).within)
        ) {
            throw ObjectError.of(
                "RECOVERY_BLOCKED",
                `cannot restore ${object.name}: it is past its recovery window`,
            );
        }

        // restore it under a live parent, with the children trashed with it
        const row: Row = target;
        const parentId = row["parentId"];
        await ObjectRecord.requireParentReady(
            context,
            parentId === undefined || parentId === null ? null : schema.string().parse(parentId),
        );
        const restored = await context.update({
            deletedAt: null,
            ...ObjectReference.columns("deletedBy", undefined),
        });
        await ObjectRecord.trashChildren(context, schema.string().parse(row["id"]), "restore");

        return restored;
    },

    /** Purge an object in the trash, removing it at the loaded revision. */
    async purge(
        context: MethodContext<{ readonly table: RecoverableTable }, unknown>,
    ): Promise<Record<string, never>> {
        Recoverable.requireTrashed(context);
        await context.remove();

        return {};
    },

    /** Purge a retained record in the trash, marking its content destroyed. */
    async purgeRetained(
        context: MethodContext<{ readonly table: RetainedTable }, unknown>,
    ): Promise<Record<string, never>> {
        Recoverable.requireTrashed(context);
        await context.update({ purgedAt: context.now });

        return {};
    },

    /** Require an object in the trash, purged at most once. */
    requireTrashed(context: MethodContext<{ readonly table: RecoverableTable }, unknown>): void {
        const { object } = context;
        const target = context.target;
        if (target.deletedAt === null) {
            throw ObjectError.of(
                "DELETION_BLOCKED",
                `cannot purge ${object.name}: it is not deleted`,
            );
        } else if (Recoverable.isPurged(target)) {
            throw ObjectError.of(
                "PURGE_BLOCKED",
                `cannot purge ${object.name}: it is already purged`,
            );
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

/** Declare the time a record was deleted and who deleted it. */
function deletionColumns() {
    return {
        /** The time the record was deleted, null otherwise. */
        deletedAt: integer("deleted_at"),
        /** The package of the subject that deleted the record, null otherwise. */
        deletedByPackageId: identifier("deleted_by_package_id", "package"),
        /** The type of the subject that deleted the record. */
        deletedByType: text("deleted_by_type"),
        /** The scope of the subject that deleted the record. */
        deletedByScope: text("deleted_by_scope"),
        /** The identifier of the subject that deleted the record. */
        deletedById: text("deleted_by_id"),
    };
}

/** The columns of a record deleted to a trash. */
export type DeletionColumns = ReturnType<typeof deletionColumns>;
