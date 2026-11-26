import { type Allowance, USAGE_STATES, type UsageState } from "../feature/allowance.ts";
import { Identifier, type JsonValue, nth, present, schema, Text } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import type { Period } from "../rate/period.ts";

/** One grant's allotment of a feature: what it grants, with its metered usage. */
export type Allotment = Pick<
    Allowance,
    "packageId" | "feature" | "kind" | "value" | "limit" | "resetAt" | "state" | "resource"
> & {
    /** The usage of a metered feature since its last reset, null for another kind. */
    readonly usage: number | null;
    /** The subscription item or default product granting the feature. */
    readonly grant: ObjectReference;
    /** When the grant was created, the latest granting a static feature's value. */
    readonly grantedAt: number;
    /** The grant's current billing period. */
    readonly period: Period;
};

/** What an account may use of a feature across the grants granting it, in its billing period. */
export type Resolution = Pick<Allowance, "packageId" | "feature" | "resource"> & {
    /** The start of the billing period ending first among the grants granting it. */
    readonly periodStart: number;
} & (
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
              /** The sum of the grants' limits, unlimited when any grant is. */
              readonly limit: number | null;
              /** The usage within the current period of the grant resetting first. */
              readonly usage: number;
              /** The reset of the grant resetting first, none when no grant resets. */
              readonly resetAt: number | null;
              /** The least advanced state of the grants, as any grant with room allows usage. */
              readonly state: UsageState;
          }
    );

/** The allotments of features. */
export const Allotment = {
    /** Resolve a feature's effective grant across its grants' allotments. */
    resolve(allotments: readonly Allotment[]): Resolution {
        // require one kind among the feature's allotments
        const first = nth(allotments, 0);
        if (allotments.some((row) => row.kind !== first.kind)) {
            throw new TypeError(`allotments of '${first.feature}' have different kinds`);
        }

        // resolve in the billing period ending first among the grants
        const ending = nth(
            allotments.toSorted((left, right) => left.period.end - right.period.end),
            0,
        );
        const { packageId, feature, resource } = first;
        const resolved = { packageId, feature, resource, periodStart: ending.period.start };

        // grant access when any grant does
        if (first.kind === "boolean") {
            return { ...resolved, kind: "boolean" };
        }
        // take the highest numeric value, or else the most recently granted one
        else if (first.kind === "static") {
            return { ...resolved, kind: "static", value: resolveValue(allotments) };
        }

        // add the limits up, unlimited when any grant is
        const limits = allotments.flatMap((row) => (row.limit === null ? [] : [row.limit]));
        const limit =
            limits.length < allotments.length ? null : limits.reduce((sum, each) => sum + each, 0);

        // count the usage within the period of the grant resetting first
        const resetting = nth(
            allotments.toSorted(
                (left, right) => (left.resetAt ?? Infinity) - (right.resetAt ?? Infinity),
            ),
            0,
        );

        // take the least advanced state among the grants
        const states = allotments.map((row) => USAGE_STATES.indexOf(present(row.state, "a state")));
        const state = present(USAGE_STATES[Math.min(...states)], "a usage state");

        return {
            ...resolved,
            kind: "metered",
            limit,
            usage: present(resetting.usage, "a metered usage"),
            resetAt: resetting.resetAt,
            state,
        };
    },
};

/** Resolve static allotments: the highest number, or else the latest granted value. */
function resolveValue(allotments: readonly Allotment[]): JsonValue {
    // take the highest number when every value is one
    const values = allotments.map((row) => present(row.value, "a static value"));
    const numbers = values.filter((value) => typeof value === "number");
    if (numbers.length === values.length) {
        return Math.max(...numbers);
    }

    // take the value of the latest grant, by its time-ordered identifier on equal grant times
    const latest = allotments.toSorted(
        (left, right) =>
            right.grantedAt - left.grantedAt ||
            Text.compare(grantOrder(right.grant.id), grantOrder(left.grant.id)),
    );

    return present(nth(latest, 0).value, "a static value");
}

/** Read the time-ordered part of a grant's identifier, its UUIDv7 after its type's prefix. */
function grantOrder(grantId: string): string {
    return Identifier.uuid(schema.anyIdentifier().parse(grantId));
}
