import { Bucket, type DatabaseConnection } from "@destack/db";
import { canonicalize, Duration, type JsonValue, schema } from "@destack/schema";
import type { Subject } from "@destack/sync";

/** An object of an input, whose dotted paths key limits. */
const INPUT_OBJECT = schema.record(schema.string(), schema.json());

/** Whose limit a call or run counts against: its scope's, its caller's, or an input value's. */
export type LimitKey = "scope" | "caller" | { readonly input: string };

/** How often calls or runs start per key, and how many runs advance at once. */
export interface Limits {
    /** How many start per period, and at most at once after idling. */
    readonly rate?: { readonly count: number; readonly every: Duration; readonly burst?: number };
    /** How many advance at once, for workflows' runs. */
    readonly concurrency?: { readonly slots: number };
    /** Whose limit each counts against, its scope's by default. */
    readonly key?: LimitKey;
}

/** How often a method runs per key: its rate and whose limit a call counts against. */
export type MethodLimits = Pick<Limits, "rate" | "key">;

/** What a limit is keyed by: the scope, the caller and the input. */
export interface LimitSubject {
    /** The scope the call or run acts in. */
    readonly scope: string;
    /** The principal the call or run acts as. */
    readonly caller: Subject;
    /** The call's or run's input. */
    readonly input: JsonValue;
}

/** The limits of methods and workflows, counted in the database's buckets and leases. */
export const Limits = {
    /** Read whose limit a call or run counts against. */
    key(key: LimitKey | undefined, subject: LimitSubject): string {
        // count against the scope by default, else the caller or an input value
        if (key === undefined || key === "scope") {
            return subject.scope;
        } else if (key === "caller") {
            return canonicalize(subject.caller);
        } else {
            return canonicalize(Limits.read(subject.input, key.input));
        }
    },

    /** Take a token of a rate for a key, answering how long until one is due once limited. */
    async take(
        database: DatabaseConnection,
        name: string,
        key: string,
        rate: NonNullable<Limits["rate"]>,
        now: number,
    ): Promise<number | undefined> {
        // take a token counted in the rate's current period
        const every = Duration.milliseconds(rate.every);
        const taken = await Bucket.take(database, {
            name,
            key,
            rate: { count: rate.count, every },
            burst: rate.burst ?? rate.count,
            now,
        });

        return taken.kind === "limited" ? taken.retryAfter : undefined;
    },

    /** Read a dotted path of an input, refusing a missing one. */
    read(input: JsonValue, path: string): JsonValue {
        let value: JsonValue = input;
        for (const part of path.split(".")) {
            const parsed = INPUT_OBJECT.safeParse(value);
            const entry = parsed.success ? parsed.data[part] : undefined;
            if (entry === undefined) {
                throw new TypeError(`an input has no ${path} to key its limit by`);
            }
            value = entry;
        }

        return value;
    },
};
