import { Duration, type schema } from "@destack/schema";
import { type ColumnBuilder, identifier, integer, text } from "@destack/db";
import type { FieldColumn } from "../field/field.ts";
import type { Method } from "../method/method.ts";
import { method } from "../method/record.ts";
import {
    Empty,
    type Procedure,
    type ReplayFields,
    type RowSchema,
    type TargetFields,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

import { RecoverableMethod } from "../method/recoverable.ts";
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
    ? ReturnType<typeof deletionColumns> &
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

/** Declare the time a record was deleted and who deleted it. */
export function deletionColumns() {
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

/** Objects deleted to a trash, restorable within a window and purged after it. */
export const recoverable: Trait<RecoverableDefinition> = {
    key: "recoverable",
    storages: ["durable"],
    options: (definition) => definition.recoverable,
    columns: (options) => ({
        ...deletionColumns(),
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
        restore: RecoverableMethod.restoration("restore", options.by, options),
        purge: RecoverableMethod.restoration("purge", options.purge ?? options.by, options),
        ...(options.retain === "record"
            ? { discard: RecoverableMethod.discard(options.purge ?? options.by) }
            : {}),
    }),
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

/** The procedures recoverable deletion derives besides deleting. */
export type RecoverableProcedures<Object extends ObjectType> = {
    restore: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object>>,
        RowSchema<Object>
    >;
    purge: Procedure<schema.Object<TargetFields<Object> & ReplayFields<Object>>, schema.Object<{}>>;
};
