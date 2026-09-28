import { earliest } from "@destack/access";
import {
    and,
    type Column,
    type ColumnBuilder,
    type DatabaseConnection,
    eq,
    integer,
    isNotNull,
    isNull,
    lte,
    min,
    type Table,
} from "@destack/db";
import type { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Call } from "../method/call.ts";
import { defineMethod, type Method } from "../method/method.ts";
import { Step } from "../method/step.ts";
import {
    Empty,
    type Procedure,
    type ReplayShape,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import { Duration } from "../object/duration.ts";
import type { ObjectType } from "../object/object.ts";
import type { Controller } from "@destack/service/control";
import { type ObjectServer, SystemCall } from "../server/server.ts";
import { remove } from "./record.ts";
import type { Trait } from "./trait.ts";

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
};

/** The columns recording the deletion request and the purge of a kept record's content. */
export type DeletionBuilderMap<Deletion> = Deletion extends RecoverableDefinition
    ? { deletionRequestedAt: ColumnBuilder<number, false, false> } & (Deletion extends {
          readonly keep: "record";
      }
          ? { purgedAt: ColumnBuilder<number, false, false> }
          : {})
    : {};

/** The methods recoverable deletion derives. */
export type RecoverableMethodMap<Deletion> = Deletion extends RecoverableDefinition
    ? {
          readonly delete: Method<"delete", Deletion["by"], never, never, true>;
          readonly restore: Method<"restore", Deletion["by"], never, never, true>;
          readonly purge: Method<"purge", PurgePermission<Deletion>, never, never, true>;
      }
    : {};

/** The permission purging needs: the one `purge` names, else `by`. */
type PurgePermission<Deletion extends RecoverableDefinition> = Deletion extends {
    readonly purge: infer Purge extends string;
}
    ? Purge
    : Deletion["by"];

/** Declare the time a record's deletion was requested. */
export function deletionColumns() {
    return {
        /** The time deletion was requested, null otherwise. */
        deletionRequestedAt: integer("deletion_requested_at"),
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
        delete: remove(options.by),
        restore: restoration("restore", options.by, true),
        purge: restoration("purge", options.purge ?? options.by, options.keep !== "record"),
    }),
    validate: (options, object, definition) => {
        // refuse controlled objects
        if (definition.controlled) {
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
        const declared = object.recoverable;
        if (declared === undefined) {
            throw new TypeError(`object ${object.name} is not recoverable`);
        }
        Duration.require(within, `recovery window of ${object.name}`);

        // copy the type with the window
        const options: RecoverableDefinition = { ...declared, within };
        const traits = object.traits.map((applied) =>
            applied.trait === recoverable ? { trait: applied.trait, options } : applied,
        );

        return object.with({ recoverable: options, traits });
    },
    async purge(server, now) {
        // purge each type in batches until one comes back short
        let purged = 0;
        for (const object of server.objects) {
            for (
                let batch = PURGE_ROWS;
                object.recoverable !== undefined && batch === PURGE_ROWS;
            ) {
                // purge one batch
                const rows = await expired(server.database, object, now);
                if (rows.length > 0) {
                    await server.executeAsSystem(
                        object,
                        purgeMethod(object),
                        rows.map(SystemCall.of),
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
        const types = server.objects.filter((object) => object.recoverable !== undefined);

        return {
            name: "purge",
            watches: types.map((object) => object.table as Table),
            keys: (change) => {
                // look again when a row enters the trash
                const after = change.after as { deletionRequestedAt?: number | null } | undefined;
                const before = change.before as { deletionRequestedAt?: number | null } | undefined;
                const deleted = after?.deletionRequestedAt ?? null;

                return deleted !== null && before?.deletionRequestedAt !== deleted ? ["trash"] : [];
            },
            list: async () => ["trash"],
            reconcile: async () => {
                // purge, then schedule the earliest window end
                const now = Date.now();
                await recoverable.purge(server, now);
                const ends = await Promise.all(
                    types.map((object) => ending(server.database, object)),
                );
                const next = earliest(ends);

                return next === undefined ? undefined : Math.max(0, next - now);
            },
        };
    },
};

/** Read a batch of a type's expired, unpurged deleted rows. */
async function expired(
    database: DatabaseConnection,
    object: ObjectType,
    now: number,
): Promise<Record<string, unknown>[]> {
    // read expired rows
    const table = object.table as Table & Record<string, Column>;
    const window = Duration.milliseconds(object.recoverable!.within);
    const isKept = object.recoverable!.keep === "record";

    return (await database
        .select()
        .from(table)
        .where(
            and(
                lte(table.deletionRequestedAt!, now - window),
                isKept ? isNull(table.purgedAt!) : undefined,
            ),
        )
        .limit(PURGE_ROWS)) as Record<string, unknown>[];
}

/** Read when a type's earliest pending deletion leaves its window. */
async function ending(
    database: DatabaseConnection,
    object: ObjectType,
): Promise<number | undefined> {
    // read the earliest unpurged deletion request
    const table = object.table as Table & Record<string, Column>;
    const isKept = object.recoverable!.keep === "record";
    const [row] = await database
        .select({ requestedAt: min(table.deletionRequestedAt!) })
        .from(table)
        .where(
            and(
                isNotNull(table.deletionRequestedAt!),
                isKept ? isNull(table.purgedAt!) : undefined,
            ),
        );
    const requestedAt = row?.requestedAt as number | null | undefined;

    return requestedAt == null
        ? undefined
        : requestedAt + Duration.milliseconds(object.recoverable!.within);
}

/** Name the purge method a recoverable type takes. */
function purgeMethod(object: ObjectType): string {
    return Object.entries(object.methods as Readonly<Record<string, Method>>).find(
        ([, declared]) => declared.kind === "purge",
    )![0];
}

/** Restore a deleted object, or purge it for good. */
function restoration(
    kind: "restore" | "purge",
    permission: string,
    isPredicted: boolean,
): Method<"restore" | "purge", string, never, never, true> {
    const isRestore = kind === "restore";

    return defineMethod<Method<"restore" | "purge", string, never, never, true>>({
        kind,
        permission,
        mutates: true,
        isPredicted,
        target: true,
        result: isRestore ? "object" : "value",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: `/{id}/${kind}` },
            input: shapes.target.extend(shapes.replay),
            output: isRestore ? shapes.row : Empty,
        }),
        effect: isRestore ? restore : purge,
        ...(isRestore
            ? {
                  inverse: (step: Step) => {
                      // delete the restored object to its trash again
                      const call = Step.call(step, "delete", Step.target(step, step.input.id));

                      return call === undefined ? undefined : [call];
                  },
              }
            : {
                  async execute(this: Method, call: Call) {
                      // require a purge handler for kept records
                      if (call.object.recoverable!.keep === "record" && this.effect === purge) {
                          throw new TypeError(
                              `object ${call.object.name} keeps purged records, so a purge handler of its own destroys their content`,
                          );
                      }

                      return this.effect(call);
                  },
              }),
    });
}

/** Clear a deletion that is still recoverable. */
async function restore(call: Call): Promise<Record<string, unknown>> {
    // require an unpurged deletion within its window
    const { object } = call;
    const target = call.target as Record<string, unknown>;
    const requested = target.deletionRequestedAt as number | null;
    if (requested === null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is not deleted` });
    } else if (isPurged(target)) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is purged` });
    } else if (call.now - requested >= Duration.milliseconds(object.recoverable!.within)) {
        throw new ServiceError("CONFLICT", {
            message: `${object.name} is past its recovery window`,
        });
    }

    return call.revise({ deletionRequestedAt: null });
}

/** Purge an object in the trash: mark a kept record purged, else remove it. */
async function purge(call: Call): Promise<Record<string, never>> {
    // purge only objects in the trash, once
    const { object } = call;
    const target = call.target as Record<string, unknown>;
    if (target.deletionRequestedAt === null) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is not deleted` });
    } else if (isPurged(target)) {
        throw new ServiceError("CONFLICT", { message: `${object.name} is purged` });
    }

    // mark a kept record purged
    if (object.recoverable!.keep === "record") {
        await call.revise({ purgedAt: call.now });
    }
    // remove the object
    else {
        const table = object.table as Table & Record<string, never>;
        await call.database.delete(table).where(eq(table.id, target.id));
    }

    return {};
}

/** Decide whether a purge already destroyed a kept record's content. */
function isPurged(row: Readonly<Record<string, unknown>>): boolean {
    return row.purgedAt !== undefined && row.purgedAt !== null;
}

/** The procedures recoverable deletion derives besides deleting. */
export type RecoverableProcedures<Object extends ObjectType> = {
    restore: Procedure<schema.Object<TargetShape<Object> & ReplayShape<Object>>, RowSchema<Object>>;
    purge: Procedure<schema.Object<TargetShape<Object> & ReplayShape<Object>>, schema.Object<{}>>;
};
