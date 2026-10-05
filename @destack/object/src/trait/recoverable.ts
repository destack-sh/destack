import { Duration, type schema } from "@destack/schema";
import { type ColumnBuilder, integer, text } from "@destack/db";
import type { FieldColumn } from "../field/field.ts";
import type { Call } from "../method/call.ts";
import { method, type Method } from "../method/method.ts";
import {
    Empty,
    type Procedure,
    type ReplayShape,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

import { discard, restoration } from "../method/recoverable.ts";
import type { TraitTable } from "../object/table.ts";
/** The options of recoverable deletion. */
export type RecoverableDefinition<Permissions extends string = string> = {
    /** How long deleted objects stay restorable. */
    readonly within: Duration;
    /** The permission deleting, restoring and by default purging the objects. */
    readonly by: Permissions;
    /** The object permission whose holders purge the objects, `by` when absent. */
    readonly purge?: Permissions;
    /** What a purge keeps, the record or nothing. */
    readonly keep?: "record";
    /** The external work a purge prepares before its transaction, such as releasing storage. */
    readonly prepared?: schema.Schema;
};

/** The columns recording the deletion request and the purge of a kept record's content. */
export type DeletionBuilderMap<Deletion> = Deletion extends RecoverableDefinition
    ? {
          deletionRequestedAt: ColumnBuilder<FieldColumn<number, false, false>>;
          deletedBy: ColumnBuilder<FieldColumn<string, false, false>>;
      } & (Deletion extends {
          readonly keep: "record";
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
      } & (Deletion extends { readonly keep: "record" }
          ? {
                readonly discard: Method<{
                    kind: "custom";
                    permission: null;
                    output: typeof Empty;
                    mutates: true;
                }>;
            }
          : {})
    : {};

/** The permission purging needs: the one `purge` names, else `by`. */
type PurgePermission<Deletion extends RecoverableDefinition> = Deletion extends {
    readonly purge: infer Purge extends string;
}
    ? Purge
    : Deletion["by"];

/** The external work a purge prepares: the schema `prepared` names, else none. */
type PurgePrepared<Deletion extends RecoverableDefinition> = Deletion extends {
    readonly prepared: infer Prepared extends schema.Schema;
}
    ? Prepared
    : never;

/** Declare the time a record's deletion was requested. */
export function deletionColumns() {
    return {
        /** The time deletion was requested, null otherwise. */
        deletionRequestedAt: integer("deletion_requested_at"),
        /** The subject key of the caller that requested deletion, absent otherwise. */
        deletedBy: text("deleted_by"),
    };
}

/** Objects deleted to a trash, restorable within a window and purged after it. */
export const recoverable: Trait<RecoverableDefinition> & {
    /** Copy a recoverable object type with the recovery window its host sets. */
    within<Self extends ObjectType>(object: Self, within: Duration): Self;
} = {
    key: "recoverable",
    isDurable: true,
    options: (definition) => definition.recoverable,
    columns: (options) => ({
        ...deletionColumns(),
        ...(options.keep === "record"
            ? {
                  /** The time a purge destroyed the kept record's content, null before. */
                  purgedAt: integer("purged_at"),
              }
            : {}),
    }),
    constraints: () => [],
    methods: (options) => ({
        delete: method.delete(options.by),
        restore: restoration("restore", options.by, options),
        purge: restoration("purge", options.purge ?? options.by, options),
        ...(options.keep === "record" ? { discard } : {}),
    }),
    validate: (options, object, definition) => {
        // refuse controlled objects
        if (definition.controlled !== undefined) {
            throw new TypeError(
                `object ${object.name} is controlled, so its controller finishes its deletion`,
            );
        }

        // validate the window and keep
        Duration.require(options.within, `recovery window of ${object.name}`);
        if (options.keep !== undefined && options.keep !== "record") {
            throw new TypeError(`purges of ${object.name} keep the record or nothing`);
        }
    },
    within(object, within) {
        // require a recoverable type and a window
        const declared = object.lifecycle.recoverable;
        if (declared === undefined) {
            throw new TypeError(`object ${object.name} is not recoverable`);
        }
        Duration.require(within, `recovery window of ${object.name}`);

        // copy the type with the window
        const options: RecoverableDefinition = { ...declared, within };
        const traits = object.traits.map((applied) =>
            applied.trait === recoverable ? { trait: applied.trait, options } : applied,
        );

        return object.with({ lifecycle: { ...object.lifecycle, recoverable: options }, traits });
    },
};

/** Read a call as its object's kept table types it, absent for an object purging its whole record. */
export function keptCall(
    call: Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>>,
):
    | Call<
          TraitTable<{ readonly recoverable: RecoverableDefinition & { readonly keep: "record" } }>
      >
    | undefined;
/**
 * Read a call as its object's kept table types it, by the trait's option.
 *
 * @construct defineObject derives the purge column into the table of every object whose recovery keeps the record.
 */
export function keptCall(
    call: Call<TraitTable<{ readonly recoverable: RecoverableDefinition }>>,
): Call | undefined {
    return call.object.lifecycle.recoverable?.keep === "record" ? call : undefined;
}

/** Read an object type as its recoverable table types it, absent for a type without the trait. */
export function recoverableType(
    object: ObjectType,
): ObjectOf<{ table: TraitTable<{ readonly recoverable: RecoverableDefinition }> }> | undefined;
/**
 * Read an object type as its recoverable table types it, by the trait's option.
 *
 * @construct defineObject derives the trait's deletion columns into the table of every object whose definition sets `recoverable`.
 */
export function recoverableType(object: ObjectType): ObjectType | undefined {
    return object.lifecycle.recoverable === undefined ? undefined : object;
}

/** The procedures recoverable deletion derives besides deleting. */
export type RecoverableProcedures<Object extends ObjectType> = {
    restore: Procedure<schema.Object<TargetShape<Object> & ReplayShape<Object>>, RowSchema<Object>>;
    purge: Procedure<schema.Object<TargetShape<Object> & ReplayShape<Object>>, schema.Object<{}>>;
};
