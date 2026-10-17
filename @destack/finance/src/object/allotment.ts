import { type Entitlement, USAGE_STATES, type UsageState } from "@destack/account/object";
import { Identifier, type JsonValue, nth, present, schema, Text } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import type { Period } from "../rate/period.ts";

/** One source's allotment of a feature: what it grants, with its metered usage. */
export type Allotment = Pick<
    Entitlement,
    "packageId" | "feature" | "kind" | "value" | "limit" | "usage" | "resetAt" | "state"
> & {
    /** The subscription item or default product granting the feature. */
    readonly source: ObjectReference;
    /** When the source was created, the latest granting a static feature's value. */
    readonly grantedAt: number;
    /** The source's current billing period. */
    readonly period: Period;
};

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
          /** The least advanced state of the sources, since any source with room allows usage. */
          readonly state: UsageState;
      };

/** The allotments of features. */
export const Allotment = {
    /** Resolve a feature's effective grant across its sources' allotments. */
    resolve(allotments: readonly Allotment[]): Resolution {
        // require one kind among the feature's allotments
        const first = nth(allotments, 0);
        if (allotments.some((row) => row.kind !== first.kind)) {
            throw new TypeError(`allotments of ${first.feature} disagree on its kind`);
        }

        // grant access when any source does
        if (first.kind === "boolean") {
            return { kind: "boolean" };
        }
        // take the highest numeric value, or else the most recently granted one
        else if (first.kind === "static") {
            return { kind: "static", value: resolveValue(allotments) };
        }

        // add the limits up, unlimited when any source is
        const limits = allotments.flatMap((row) => (row.limit === null ? [] : [row.limit]));
        const limit =
            limits.length < allotments.length ? null : limits.reduce((sum, each) => sum + each, 0);

        // count the usage within the period of the source resetting first
        const resetting = nth(
            allotments.toSorted(
                (left, right) => (left.resetAt ?? Infinity) - (right.resetAt ?? Infinity),
            ),
            0,
        );

        // take the least advanced state among the sources
        const states = allotments.map((row) => USAGE_STATES.indexOf(present(row.state, "a state")));
        const state = present(USAGE_STATES[Math.min(...states)], "a usage state");

        return {
            kind: "metered",
            limit,
            usage: present(resetting.usage, "a metered usage"),
            resetAt: resetting.resetAt,
            state,
        };
    },

    /** Read the start of the billing period ending first among a feature's allotments. */
    periodStart(allotments: readonly Allotment[]): number {
        const ending = nth(
            allotments.toSorted((left, right) => left.period.end - right.period.end),
            0,
        );

        return ending.period.start;
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

    // take the value of the latest source, by its time-ordered identifier on equal grant times
    const latest = allotments.toSorted(
        (left, right) =>
            right.grantedAt - left.grantedAt ||
            Text.compare(sourceOrder(right.source.id), sourceOrder(left.source.id)),
    );

    return present(nth(latest, 0).value, "a static value");
}

/** Read the time-ordered part of a source's identifier, its UUIDv7 after its type's prefix. */
function sourceOrder(sourceId: string): string {
    return Identifier.uuid(schema.anyIdentifier().parse(sourceId));
}
