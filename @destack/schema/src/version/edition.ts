import * as z from "zod";
import { canonicalize } from "../json/json.ts";

/** The edition of a schema no conversion has changed yet. */
const FIRST = 1;

/** A schema's edition: 1 as first declared, then one more with each conversion its author adds. */
export const Edition = Object.assign(z.number().int().min(FIRST), {
    /** Read the edition conversions lead to: the highest one keyed, else the first. */
    of(...keyed: readonly Readonly<Record<number, unknown>>[]): number {
        const editions = keyed.flatMap((each) => Object.keys(each).map(Number));

        return Math.max(FIRST, ...editions);
    },

    /** List the entries keyed by editions after one up to another, in order. */
    between<Value>(
        keyed: Readonly<Record<number, Value>>,
        after: number,
        upto: number,
    ): [number, Value][] {
        return Object.entries(keyed)
            .map(([edition, value]): [number, Value] => [Number(edition), value])
            .filter(([edition]) => edition > after && edition <= upto)
            .toSorted(([left], [right]) => left - right);
    },

    /** List the editions whose conversions an earlier declaration has and a later one changes. */
    changed(
        earlier: Readonly<Record<number, unknown>>,
        later: Readonly<Record<number, unknown>>,
    ): number[] {
        return Object.keys(earlier)
            .map(Number)
            .filter(
                (edition) =>
                    canonicalize(earlier[edition]) !== canonicalize(later[edition] ?? null),
            )
            .toSorted((left, right) => left - right);
    },

    /** Refuse conversion keys that are no edition after the first. */
    require(keyed: Readonly<Record<number, unknown>>, name: string): void {
        for (const key of Object.keys(keyed)) {
            const edition = Number(key);
            if (!Number.isInteger(edition) || edition <= FIRST) {
                throw new TypeError(
                    `conversion key of '${name}' is no edition after ${FIRST}: '${key}'`,
                );
            }
        }
    },
});

/** A schema's edition. */
export type Edition = z.infer<typeof Edition>;
