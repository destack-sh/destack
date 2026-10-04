import { earliest } from "@destack/access";
import { Duration, present, type schema } from "@destack/schema";
import {
    Change,
    and,
    type ColumnBuilder,
    type DatabaseConnection,
    integer,
    isNotNull,
    isNull,
    lte,
    min,
    type Row,
    text,
    type Select,
    TABLE,
} from "@destack/db";
import type { FieldColumn } from "../field/field.ts";
import { ServiceError } from "@destack/service/error";
import type { Call } from "../method/call.ts";
import { defineMethod, method, type Method, type MethodBuilder } from "../method/method.ts";
import { Step } from "../method/step.ts";
import {
    Empty,
    type Procedure,
    type ReplayShape,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { Controller } from "@destack/service/control";
import { type ObjectServer, SystemCall } from "../server/server.ts";
import type { Trait } from "./trait.ts";
import type { ObjectTable } from "../object/table.ts";

/** The most rows one purge removes per transaction: about a millisecond of deletes. */
const PURGE_ROWS = 100;

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

/** The table of recoverable objects: the record columns and the deletion request. */
export type RecoverableTable = ObjectTable<
    string,
    unknown,
    {},
    { readonly recoverable: RecoverableDefinition }
>;

/** The table of recoverable objects whose purge keeps the record. */
export type KeptTable = ObjectTable<
    string,
    unknown,
    {},
    { readonly recoverable: RecoverableDefinition & { readonly keep: "record" } }
>;

/** The method declarations of recoverable objects whose purge keeps the record. */
const keptMethod: MethodBuilder<KeptTable> = method;

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
    /** Purge expired deleted objects as the system, returning the count. */
    purge(
        server: Pick<ObjectServer, "objects" | "database" | "executeAsSystem">,
        now: number,
    ): Promise<number>;
    /** Purge deleted objects as their recovery windows end. */
    controller(server: Pick<ObjectServer, "objects" | "database" | "executeAsSystem">): Controller;
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
    async purge(server, now) {
        // purge each type in batches until one comes back short
        let purged = 0;
        for (const { object, options } of recoverables(server.objects)) {
            for (let batch = PURGE_ROWS; batch === PURGE_ROWS;) {
                // purge one batch
                const rows = await expired(server.database, object, options, now);
                if (rows.length > 0) {
                    await server.executeAsSystem(
                        object,
                        purgeMethod(object),
                        rows.map((row) => SystemCall.of(row)),
                        now,
                    );
                }
                batch = rows.length;
                purged += batch;
            }
        }

        return purged;
    },
    controller(server) {
        // watch recoverable objects
        const types = recoverables(server.objects);

        return {
            name: "purge",
            watches: types.map(({ object }) => object.table),
            keys: (change) => {
                // look again when a row enters the trash
                const deleted = Change.after(change)?.["deletionRequestedAt"] ?? null;

                return deleted !== null &&
                    Change.before(change)?.["deletionRequestedAt"] !== deleted
                    ? ["trash"]
                    : [];
            },
            list: async () => ["trash"],
            reconcile: async () => {
                // purge, then schedule the earliest window end
                const now = Date.now();
                await recoverable.purge(server, now);
                const ends = await Promise.all(
                    types.map(({ object, options }) => ending(server.database, object, options)),
                );
                const next = earliest(ends);

                return next === undefined ? undefined : Math.max(0, next - now);
            },
        };
    },
};

/** Read a call as its object's kept table types it, absent for an object purging its whole record. */
export function keptCall(call: Call<RecoverableTable>): Call<KeptTable> | undefined;
/**
 * Read a call as its object's kept table types it, by the trait's option.
 *
 * @construct defineObject derives the purge column into the table of every object whose recovery keeps the record.
 */
export function keptCall(call: Call<RecoverableTable>): Call | undefined {
    return call.object.lifecycle.recoverable?.keep === "record" ? call : undefined;
}

/** Read an object type as its recoverable table types it, absent for a type without the trait. */
export function recoverableType(
    object: ObjectType,
): ObjectOf<{ table: RecoverableTable }> | undefined;
/**
 * Read an object type as its recoverable table types it, by the trait's option.
 *
 * @construct defineObject derives the trait's deletion columns into the table of every object whose definition sets `recoverable`.
 */
export function recoverableType(object: ObjectType): ObjectType | undefined {
    return object.lifecycle.recoverable === undefined ? undefined : object;
}

/** Read a batch of a type's expired, unpurged deleted rows. */
async function expired(
    database: DatabaseConnection,
    object: ObjectOf<{ table: RecoverableTable }>,
    options: RecoverableDefinition,
    now: number,
): Promise<Select<RecoverableTable>[]> {
    // read expired rows, unpurged where the type keeps purged records
    const table = object.table;
    const window = Duration.milliseconds(options.within);
    const unpurged =
        options.keep === "record" ? isNull(table[TABLE].column("purgedAt")) : undefined;

    return database
        .select()
        .from(table)
        .where(and(lte(table.deletionRequestedAt, now - window), unpurged))
        .limit(PURGE_ROWS);
}

/** Read when a type's earliest pending deletion leaves its window. */
async function ending(
    database: DatabaseConnection,
    object: ObjectOf<{ table: RecoverableTable }>,
    options: RecoverableDefinition,
): Promise<number | undefined> {
    // read the earliest unpurged deletion request
    const table = object.table;
    const unpurged =
        options.keep === "record" ? isNull(table[TABLE].column("purgedAt")) : undefined;
    const [row] = await database
        .select({ requestedAt: min(table.deletionRequestedAt) })
        .from(table)
        .where(and(isNotNull(table.deletionRequestedAt), unpurged));
    if (row === undefined) {
        throw new TypeError(`the earliest deletion of ${object.name} read no row`);
    }
    const requestedAt = row.requestedAt;

    return requestedAt === null ? undefined : requestedAt + Duration.milliseconds(options.within);
}

/** Select the recoverable object types among the served ones, with their recovery options. */
function recoverables(objects: readonly ObjectType[]): {
    readonly object: ObjectOf<{ table: RecoverableTable }>;
    readonly options: RecoverableDefinition;
}[] {
    return objects.flatMap((object) => {
        const typed = recoverableType(object);

        return typed === undefined || object.lifecycle.recoverable === undefined
            ? []
            : [{ object: typed, options: object.lifecycle.recoverable }];
    });
}

/** Read the recovery a call's served object type keeps, which its host may set. */
function recoveryOf(call: Call): RecoverableDefinition {
    return present(call.object.lifecycle.recoverable, `the recovery of ${call.object.name}`);
}

/** Name the purge method a recoverable type takes. */
function purgeMethod(object: ObjectType): string {
    const [name] = present(
        Object.entries(object.methods).find(([, declared]) => declared.kind === "purge"),
        `the purge method of ${object.name}`,
    );

    return name;
}

/** Restore a deleted object, or purge it for good. */
function restoration(
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
        target: true,
        result: isRestore ? "object" : "value",
        ...(isRestore || options.prepared === undefined ? {} : { prepared: options.prepared }),
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: `/{id}/${kind}` },
            input: shapes.target.extend(shapes.replay),
            output: isRestore ? shapes.row : Empty,
        }),
        handler: isRestore ? restore : purge,
        ...(isRestore
            ? {
                  inverse: (step: Step) => {
                      // delete the restored object to its trash again
                      const call = Step.call(step, "delete", Step.target(step));

                      return call === undefined ? undefined : [call];
                  },
              }
            : {
                  async execute(this: Method, call: Call) {
                      // require a purge handler for kept records
                      if (recoveryOf(call).keep === "record" && this.handler === purge) {
                          throw new TypeError(
                              `object ${call.object.name} keeps purged records, so a purge handler of its own destroys their content`,
                          );
                      }

                      return this.handler(call);
                  },
              }),
    });
}

/** Clear a deletion that is still recoverable. */
async function restore(call: Call<RecoverableTable>): Promise<Select<RecoverableTable>> {
    // require an unpurged deletion within its window
    const { object } = call;
    const target = call.requireTarget();
    const requested = target.deletionRequestedAt;
    if (requested === null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is not deleted` });
    } else if (isPurged(target)) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is purged` });
    } else if (call.now - requested >= Duration.milliseconds(recoveryOf(call).within)) {
        throw new ServiceError("CONFLICT", {
            message: `${object.name} is past its recovery window`,
        });
    }

    return call.update({ deletionRequestedAt: null, deletedBy: null });
}

/** Purge an object in the trash: mark a kept record purged, else remove it. */
async function purge(call: Call<RecoverableTable>): Promise<Record<string, never>> {
    // purge only objects in the trash, once
    const { object } = call;
    const target = call.requireTarget();
    if (target.deletionRequestedAt === null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is not deleted` });
    } else if (isPurged(target)) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is purged` });
    }

    // mark a kept record purged
    const kept = keptCall(call);
    if (kept !== undefined) {
        await kept.update({ purgedAt: call.now });
    }
    // remove the object at the loaded revision
    else {
        await call.remove();
    }

    return {};
}

/** Remove a purged record the type keeps, at the loaded revision. */
const discard = keptMethod
    .mutation({ permission: null, isSystem: true, output: Empty })
    .handle(async (call) => {
        // require a purged record
        if (!isPurged(call.target)) {
            throw new ServiceError("CONFLICT", { message: `${call.object.name} is not purged` });
        }

        // delete it at the loaded revision
        await call.remove();

        return {};
    });

/** Decide whether a purge already destroyed a kept record's content. */
function isPurged(row: Row): boolean {
    return row["purgedAt"] !== undefined && row["purgedAt"] !== null;
}

/** The procedures recoverable deletion derives besides deleting. */
export type RecoverableProcedures<Object extends ObjectType> = {
    restore: Procedure<schema.Object<TargetShape<Object> & ReplayShape<Object>>, RowSchema<Object>>;
    purge: Procedure<schema.Object<TargetShape<Object> & ReplayShape<Object>>, schema.Object<{}>>;
};
