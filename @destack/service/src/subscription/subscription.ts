import { handleTrigger, type Handled } from "../trigger/trigger.ts";
import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, declaringModule, type ModuleMetadata } from "@destack/package";
import { type Declaration, DeclarationReference } from "@destack/package/declare";
import type { Table } from "@destack/db";
import { type ChangeRow, LogPosition } from "@destack/db/log";
import { Condition } from "@destack/db/query";

/** How long a subscription's position keeps the log's changes by default: a week, as long as common event logs retain theirs, about 6M changes of a space writing 10 a second. */
export const MAX_LAG_MILLISECONDS = 7 * 24 * 60 * 60 * 1000;

/** The changes a subscription consumes, as objects name them. */
export const SUBSCRIPTION_OPERATIONS = ["create", "update", "delete"] as const;

/** A change a subscription consumes. */
export const SubscriptionOperation = defineSchema(schema.enum(SUBSCRIPTION_OPERATIONS));
/** A change a subscription consumes. */
export type SubscriptionOperation = schema.Infer<typeof SubscriptionOperation>;

/** A subscription, as the manifest describes it. */
export const SubscriptionDescription = defineSchema(
    schema.object({
        /** The package-local subscription name. */
        name: DeclarationName,
        /** The declaration format version. */
        version: schema.literal(1),
        /** The object type whose changes the subscription consumes. */
        object: DeclarationReference,
        /** The condition the object's rows meet, over their fields. */
        where: Condition.schema.optional(),
        /** The changes consumed, each at most once. */
        on: schema.array(SubscriptionOperation).min(1),
        /** Where a new subscription starts: after the log's head, or with every row it admits there as created. */
        from: schema.enum(["now", "snapshot"]),
        /** How long the log keeps changes the subscription has not consumed, in milliseconds, after which it fails. */
        maxLag: schema.number().int().positive(),
    }),
);
/** A subscription, as the manifest describes it. */
export type SubscriptionDescription = schema.Infer<typeof SubscriptionDescription>;

/** A durable consumer of one object type's changes, which the host delivers in log order to the workload implementing it. */
export interface Subscription<Target extends Declaration = Declaration>
    extends Declaration, Handled<Subscription<Target>> {
    /** The trigger kind. */
    readonly kind: "subscription";
    /** The declaration format version. */
    readonly version: 1;
    /** The object type whose changes the subscription consumes. */
    readonly object: Target;
    /** The condition the object's rows meet, over their fields. */
    readonly where?: Condition;
    /** The changes consumed. */
    readonly on: readonly SubscriptionOperation[];
    /** Where a new subscription starts: after the log's head, or with every row it admits there as created. */
    readonly from: "now" | "snapshot";
    /** How long the log keeps changes the subscription has not consumed, in milliseconds, after which it fails. */
    readonly maxLag: number;
}

/** A row of a subscribed object type, typed by its table when the declaration carries one. */
export type SubscribedRow<Target> = Target extends {
    readonly table: infer Definition extends Table;
}
    ? ChangeRow<Definition>
    : Readonly<Record<string, unknown>>;

/**
 * One change a subscription consumes, which the host delivers once per log position.
 *
 * A host that fails between the handler and recording the run delivers the same change again, so handlers apply it through a Journal request derived from its position and key.
 */
export interface SubscriptionChange<Target = Declaration> {
    /** The change's position in the object database's log, shared only by the rows a snapshot delivers. */
    readonly position: LogPosition;
    /** The row's key, for the rows a snapshot delivers as created at one position. */
    readonly key?: Readonly<Record<string, unknown>>;
    /** The change as the subscription sees it: rows entering its condition are created, rows leaving it deleted. */
    readonly operation: SubscriptionOperation;
    /** The row before an update or deletion. */
    readonly before?: SubscribedRow<Target>;
    /** The row after a creation or update. */
    readonly after?: SubscribedRow<Target>;
}

/** Declare a subscription to an object type's changes, consumed by the implementing workload. */
export function defineSubscription<const Target extends Declaration>(
    definition: Pick<Subscription<Target>, "name" | "object" | "where" | "on"> &
        Partial<Pick<Subscription<Target>, "from" | "maxLag">>,
    module?: ModuleMetadata,
): Subscription<Target> {
    // stamp the declaring package supplied by the module transform
    const owner = declaringModule(module, "defineSubscription").package;
    const { object, ...fields } = definition;
    const description = SubscriptionDescription.omit({ object: true }).parse({
        from: "now",
        maxLag: MAX_LAG_MILLISECONDS,
        ...fields,
        version: 1,
    });

    // deliver a snapshot's rows only to subscriptions consuming creations
    if (description.from === "snapshot" && !description.on.includes("create")) {
        throw new TypeError(
            `subscription ${description.name} starts with a snapshot, whose rows are created, but consumes no creations`,
        );
    }

    return Object.freeze({
        ...description,
        kind: "subscription",
        object,
        package: owner,
        handle: handleTrigger,
    }) as Subscription<Target>;
}
