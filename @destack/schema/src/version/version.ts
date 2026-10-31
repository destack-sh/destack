import { z } from "zod";

/** A calendar version: year, month, release, and an optional dev build's sequence. */
const VERSION = /^(\d{4})\.([1-9]|1[0-2])\.(0|[1-9]\d*)(?:-dev\.(0|[1-9]\d*))?$/u;

/** A package release version, such as 2026.9.0 or 2026.9.0-dev.4. */
export const Version = Object.assign(z.string().regex(VERSION), {
    /** Order versions by release, dev builds before the release they lead to. */
    compare(left: string, right: string): number {
        // find the first differing component
        const first = components(left);
        const second = components(right);
        const differences = [
            first.year - second.year,
            first.month - second.month,
            first.release - second.release,
            first.stage - second.stage,
            first.sequence - second.sequence,
        ];

        return Math.sign(differences.find((difference) => difference !== 0) ?? 0);
    },

    /** Report whether a version is a prerelease: a dev build. */
    isPrerelease(version: string): boolean {
        return components(version).stage === 0;
    },

    /** Read the release a version leads to: itself, or a dev build's release. */
    release(version: string): string {
        const { year, month, release } = components(version);

        return `${year}.${month}.${release}`;
    },

    /** Read the next release in a version's year and month. */
    next(version: string): string {
        const { year, month, release } = components(version);

        return `${year}.${month}.${release + 1}`;
    },

    /** List the entries keyed by releases after one version's release up to another's, in order. */
    between<Value>(
        keyed: Readonly<Record<string, Value>>,
        after: string,
        upto: string,
    ): [string, Value][] {
        // compare dev builds as the release they lead to, which carries its entries
        const first = Version.release(after);
        const last = Version.release(upto);

        return Object.entries(keyed)
            .filter(
                ([release]) =>
                    Version.compare(release, first) > 0 && Version.compare(release, last) <= 0,
            )
            .toSorted(([left], [right]) => Version.compare(left, right));
    },

    /** Refuse conversion keys that are not releases or follow the declaring version's release. */
    requireUpTo<Value>(
        keyed: Readonly<Record<string, Value>>,
        version: string,
        name: string,
    ): void {
        for (const key of Object.keys(keyed)) {
            // refuse a key that is not a release or one after the declaring version's release
            if (!Version.safeParse(key).success || Version.isPrerelease(key)) {
                throw new TypeError(`conversion key of ${name} is not a release: ${key}`);
            } else if (Version.compare(key, Version.release(version)) > 0) {
                throw new TypeError(
                    `conversion of ${name} is keyed by ${key}, after its version ${version}`,
                );
            }
        }
    },
});

/** A package release version. */
export type Version = z.infer<typeof Version>;

/** Read a version's numbers, stage and dev sequence, in comparison order. */
function components(version: string) {
    // read the calendar numbers and the dev build's sequence
    const match = VERSION.exec(version);
    if (match === null) {
        throw new TypeError(`invalid version: ${version}`);
    }
    const [, year, month, release, sequence] = match;

    // sort a dev build before the release it leads to
    return {
        year: Number(year),
        month: Number(month),
        release: Number(release),
        stage: sequence === undefined ? 1 : 0,
        sequence: sequence === undefined ? 0 : Number(sequence),
    };
}
