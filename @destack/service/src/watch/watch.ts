import { handleTrigger, type Handled } from "../trigger/trigger.ts";
import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, declaringModule, type ModuleMetadata } from "@destack/package";
import { type Declaration, DeclarationReference } from "@destack/package/declare";
import type { Table } from "@destack/db";
import { type ChangeRow, LogPosition } from "@destack/db/log";
import { Condition } from "@destack/db/query";

/**
 * The default change retention of a watch, in milliseconds.
 *
 * A week matches common event logs: about 6M changes of a space writing 10 a second.
 */
export const MAX_LAG_MILLISECONDS = 7 * 24 * 60 * 60 * 1000;

/** The changes a watch consumes. */
export const WATCH_OPERATIONS = ["create", "update", "delete"] as const;

/** A change a watch consumes. */
export const WatchOperation = defineSchema(schema.enum(WATCH_OPERATIONS));
/** A change a watch consumes. */
export type WatchOperation = schema.Infer<typeof WatchOperation>;

/** A watch, as the manifest describes it. */
export const WatchDescription = defineSchema(
    schema.object({
        /** The package-local watch name. */
        name: DeclarationName,
        /** The watched object type. */
        object: DeclarationReference,
        /** The condition on the object's rows. */
        where: Condition.schema.optional(),
        /** The consumed changes. */
        on: schema.array(WatchOperation).min(1),
        /** Where a new watch starts: at the log's head, or with every matching row as created. */
        from: schema.enum(["now", "snapshot"]),
        /** How long the log keeps unconsumed changes, in milliseconds. */
        maxLag: schema.number().int().positive(),
    }),
);
/** A watch, as the manifest describes it. */
export type WatchDescription = schema.Infer<typeof WatchDescription>;

/** A durable consumer of one object type's changes. */
export interface Watch<Target extends Declaration = Declaration>
    extends Declaration, Handled<Watch<Target>> {
    /** The trigger kind. */
    readonly kind: "watch";
    /** The watched object type. */
    readonly object: Target;
    /** The condition on the object's rows. */
    readonly where?: Condition;
    /** The consumed changes. */
    readonly on: readonly WatchOperation[];
    /** Where a new watch starts: at the log's head, or with every matching row as created. */
    readonly from: "now" | "snapshot";
    /** How long the log keeps unconsumed changes, in milliseconds. */
    readonly maxLag: number;
}

/** A row of a watched object type. */
export type WatchedRow<Target> = Target extends {
    readonly table: infer Definition extends Table;
}
    ? ChangeRow<Definition>
    : Readonly<Record<string, unknown>>;

/** One change a watch consumes. */
export interface ObjectChange<Target = Declaration> {
    /** The change's position in the log. */
    readonly position: LogPosition;
    /** The row's key, for snapshot rows. */
    readonly key?: Readonly<Record<string, unknown>>;
    /** The change as the watch sees it. */
    readonly operation: WatchOperation;
    /** The row before an update or deletion. */
    readonly before?: WatchedRow<Target>;
    /** The row after a creation or update. */
    readonly after?: WatchedRow<Target>;
}

/** Declare a watch. */
export function defineWatch<const Target extends Declaration>(
    definition: Pick<Watch<Target>, "name" | "object" | "where" | "on"> &
        Partial<Pick<Watch<Target>, "from" | "maxLag">>,
    module?: ModuleMetadata,
): Watch<Target> {
    // stamp the declaring package
    const owner = declaringModule(module, "defineWatch").package;
    const { object, ...fields } = definition;
    const description = WatchDescription.omit({ object: true }).parse({
        from: "now",
        maxLag: MAX_LAG_MILLISECONDS,
        ...fields,
    });

    // require creations for a snapshot
    if (description.from === "snapshot" && !description.on.includes("create")) {
        throw new TypeError(
            `watch ${description.name} starts with a snapshot, whose rows are created, but consumes no creations`,
        );
    }

    return Object.freeze({
        ...description,
        kind: "watch",
        object,
        package: owner,
        handle: handleTrigger,
    }) as Watch<Target>;
}
