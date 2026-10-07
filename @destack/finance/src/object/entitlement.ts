import { through, union } from "@destack/access";
import { account, USAGE_STATES, type UsageState } from "@destack/account/object";
import { check, sql, uniqueIndex, type Select } from "@destack/db";
import { t } from "@destack/locale";
import { announcement } from "@destack/notification";
import { defineNotification } from "@destack/notification/declare";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { aligned, Identifier, type JsonValue, present, schema, Text } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { FEATURE_KINDS } from "../feature/feature.ts";
import { CatalogName, CatalogReference } from "../catalog/reference.ts";

/** What an account may use of a feature through a subscription item or a default product, derived by the system. */
export const entitlement = defineObject({
    name: "entitlement",
    plural: "entitlements",
    scope: account,
    fields: {
        /** The package declaring the feature. */
        packageId: field.string(PackageId),
        /** The feature's name in its package. */
        feature: field.string(CatalogName),
        /** What the feature grants. */
        kind: field.enum(FEATURE_KINDS),
        /** The value of a static feature. */
        value: field.json(schema.json()).optional(),
        /** The usage limit of a metered feature, unlimited without one. */
        limit: field.integer().optional(),
        /** The usage of a metered feature since its last reset. */
        usage: field.number().optional(),
        /** When the usage of a metered feature starts again, absent for usage that never resets. */
        resetAt: field.time().optional(),
        /** Where the usage of a metered feature stands against its limit and budget. */
        state: field.enum(USAGE_STATES).optional(),
        /** The subscription item or default product granting the feature. */
        source: field.json(ObjectReference),
    },
    permissions: {
        // the account's readers, and the workloads serving its residency, such as the cells of its spaces
        read: union(through("account", "read"), through("account", "serve")),
    },
    attachments: [announcement.attach({ by: "read" })],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
        update: method.update(null, {
            isSystem: true,
            fields: ["value", "limit", "usage", "resetAt", "state"],
        }),
        delete: method.delete(null, { isSystem: true }),
    }),
    constraints: (entry) => [
        uniqueIndex("entitlement_unique").on(
            entry.scope,
            entry.source,
            entry.packageId,
            entry.feature,
        ),
        check(
            "entitlement_kind",
            sql`(${entry.kind} = 'metered') = (${entry.usage} IS NOT NULL)
            AND (${entry.kind} = 'metered') = (${entry.state} IS NOT NULL)
            AND (${entry.kind} = 'metered' OR ${entry.limit} IS NULL AND ${entry.resetAt} IS NULL)
            AND (${entry.kind} = 'static' OR ${entry.value} IS NULL)`,
        ),
    ],
});
/** A persisted entitlement. */
export type Entitlement = Select<typeof entitlement.table>;

/** An account's usage of a feature nears or reaches its limit or its subscription's budget. */
export const usageLimit = defineNotification({
    name: "usageLimit",
    title: "Usage limits",
    description: "Your account's usage nears or reaches its plan's allowance or spending limit.",
    payload: entitlement.rowSchema().pick({ feature: true, state: true, usage: true, limit: true }),
    interruption: "timeSensitive",
    preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
    content: (payload) => ({
        title:
            payload.state === "near"
                ? t`${payload.feature} at 80 percent`
                : payload.state === "over"
                  ? t`${payload.feature} past the plan's included usage`
                  : t`${payload.feature} stopped at its limit`,
        body:
            payload.state === "blocked"
                ? t`Writes are refused and background work pauses until the plan changes or the next period starts. Reads and exports keep working.`
                : t`Usage past the included usage bills at the listed unit prices.`,
    }),
    summary: (count) => t`${count} usage limits`,
});

/** What an account may use of a feature across every source granting it. */
export type Resolution =
    | {
          /** Access alone. */
          readonly kind: "boolean";
      }
    | {
          /** A fixed value. */
          readonly kind: "static";
          /** The highest numeric value, or else the most recently granted one. */
          readonly value: JsonValue;
      }
    | {
          /** Usage of meters up to a limit. */
          readonly kind: "metered";
          /** The sum of the sources' limits, unlimited when any source is. */
          readonly limit: number | null;
          /** The usage within the current period of the source resetting first. */
          readonly usage: number;
          /** The reset of the source resetting first, none when no source resets. */
          readonly resetAt: number | null;
          /** The least advanced state among the sources, since any source with room lets usage go on. */
          readonly state: UsageState;
      };

/** The entitlements of accounts. */
export const Entitlement = {
    /** Resolve a feature's effective grant across an account's entitlements, none when no source grants it. */
    resolve(rows: readonly Entitlement[], feature: CatalogReference): Resolution | null {
        // select the feature's rows and require one kind among them
        const granted = rows.filter(
            (row) => row.packageId === feature.packageId && row.feature === feature.name,
        );
        const [first] = granted;
        if (first === undefined) {
            return null;
        } else if (granted.some((row) => row.kind !== first.kind)) {
            throw new TypeError(`entitlements of ${feature.name} disagree on its kind`);
        }

        // grant access when any source does
        if (first.kind === "boolean") {
            return { kind: "boolean" };
        }
        // take the highest numeric value, or else the most recently granted one
        else if (first.kind === "static") {
            return { kind: "static", value: resolveValue(granted) };
        }

        // add the limits up, unlimited when any source is
        const limits = granted.flatMap((row) => (row.limit === null ? [] : [row.limit]));
        const limit =
            limits.length < granted.length ? null : limits.reduce((sum, each) => sum + each, 0);

        // count the usage within the period of the source resetting first
        const resetting = aligned(
            granted.toSorted(
                (left, right) => (left.resetAt ?? Infinity) - (right.resetAt ?? Infinity),
            ),
            0,
        );

        // take the least advanced state among the sources
        const states = granted.map((row) => USAGE_STATES.indexOf(present(row.state, "a state")));
        const state = present(USAGE_STATES[Math.min(...states)], "a usage state");

        return {
            kind: "metered",
            limit,
            usage: present(resetting.usage, "a metered usage"),
            resetAt: resetting.resetAt,
            state,
        };
    },
};

/** Resolve the value of static entitlements: the highest number, or else the most recently granted value. */
function resolveValue(granted: readonly Entitlement[]): JsonValue {
    // take the highest number when every value is one
    const values = granted.map((row) => present(row.value, "a static value"));
    const numbers = values.filter((value) => typeof value === "number");
    if (numbers.length === values.length) {
        return Math.max(...numbers);
    }

    // take the value of the latest source, by its time-ordered identifier on equal derivation times
    const latest = granted.toSorted(
        (left, right) =>
            right.createdAt - left.createdAt ||
            Text.compare(sourceOrder(right.source.id), sourceOrder(left.source.id)),
    );

    return present(aligned(latest, 0).value, "a static value");
}

/** Read the time-ordered part of a source's identifier, its UUIDv7 after its type's prefix. */
function sourceOrder(sourceId: string): string {
    return Identifier.uuid(schema.anyIdentifier().parse(sourceId));
}
