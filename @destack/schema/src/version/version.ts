import { z } from "zod";
import { defineSchema } from "../declare/schema.ts";

/** The release channels building prereleases, in the order semantic versioning sorts their identifiers. */
const PRERELEASE_CHANNELS = ["dev", "nightly"] as const;

/** A calendar version: year, month and release, with an optional prerelease's channel and sequence. */
const VERSION = /^(\d{4})\.([1-9]|1[0-2])\.(0|[1-9]\d*)(?:-(dev|nightly)\.(0|[1-9]\d*))?$/u;

/** A package release version, such as 2026.9.0, the nightly build 2026.10.0-nightly.3 or the dev build 2026.9.0-dev.4. */
export const Version = Object.assign(defineSchema(z.string().regex(VERSION)), {
    /** Order two versions by release, prereleases before their release, dev builds before nightly ones. */
    compare(left: string, right: string): number {
        // find the first differing component
        const first = components(left);
        const second = components(right);
        const differences = [
            first.year - second.year,
            first.month - second.month,
            first.release - second.release,
            first.stage - second.stage,
            first.channel - second.channel,
            first.sequence - second.sequence,
        ];

        return Math.sign(differences.find((difference) => difference !== 0) ?? 0);
    },

    /** Name the release after a version's release in its year and month, such as 2026.9.1 after 2026.9.0, which its prereleases lead to. */
    next(version: string): string {
        const { year, month, release } = components(version);

        return `${year}.${month}.${release + 1}`;
    },

    /** List the entries keyed by the releases after one release and up to another, in release order. */
    between<Value>(
        keyed: Readonly<Record<string, Value>>,
        after: string,
        upto: string,
    ): [string, Value][] {
        return Object.entries(keyed)
            .filter(
                ([release]) =>
                    Version.compare(release, after) > 0 && Version.compare(release, upto) <= 0,
            )
            .toSorted(([left], [right]) => Version.compare(left, right));
    },

    /** Refuse keys that are no releases, or releases after the declaring one, such as a declaration's conversions. */
    requireUpTo<Value>(
        keyed: Readonly<Record<string, Value>>,
        release: string,
        name: string,
    ): void {
        for (const key of Object.keys(keyed)) {
            // refuse a key that is no release or one after the declaring release
            if (!Version.safeParse(key).success) {
                throw new TypeError(`conversion key of ${name} is no release: ${key}`);
            } else if (Version.compare(key, release) > 0) {
                throw new TypeError(
                    `conversion of ${name} is keyed by ${key}, after its release ${release}`,
                );
            }
        }
    },
});

/** A package release version. */
export type Version = z.infer<typeof Version>;

/** Read the calendar numbers, the stage and the prerelease's channel and sequence of a version, in comparison order. */
function components(version: string) {
    // read the calendar numbers and the prerelease
    const match = VERSION.exec(version);
    if (match === null) {
        throw new TypeError(`invalid version: ${version}`);
    }
    const [, year, month, release, channel, sequence] = match;

    // sort a prerelease before the release it leads to
    return {
        year: Number(year),
        month: Number(month),
        release: Number(release),
        stage: channel === undefined ? 1 : 0,
        channel: PRERELEASE_CHANNELS.findIndex((candidate) => candidate === channel),
        sequence: sequence === undefined ? 0 : Number(sequence),
    };
}
