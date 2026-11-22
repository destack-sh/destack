import { z } from "zod";

/** A calendar version, marked dev for a build of unreleased source. */
const VERSION = /^(\d{4})\.([1-9]|1[0-2])\.(0|[1-9]\d*)(-dev)?$/u;

/** The digits of a version's year. */
const YEAR_DIGITS = 4;

/** The version of a package no build released: the highest one, after every release. */
const DEVELOPMENT = "9999.12.0";

/** The schema of a package version, which the version's verbs write through. */
const versionSchema = z.string().regex(VERSION);

/** A package version: a release such as 2026.9.0, or a dev build's such as 2026.9.0-dev. */
export const Version = Object.assign(versionSchema, {
    /** The version of code no build released, such as code run from source: after every release. */
    development: DEVELOPMENT,

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

        return write(year, month, release);
    },

    /** Write a dev build's version, the same for every unreleased build, as Go's (devel). */
    dev(release: string): z.infer<typeof versionSchema> {
        return versionSchema.parse(`${Version.release(release)}-dev`);
    },

    /** Write the first release of the UTC month a time lies in. */
    first(at: number): string {
        const time = new Date(at);

        return `${time.getUTCFullYear()}.${time.getUTCMonth() + 1}.0`;
    },

    /** Read the next release in a version's year and month. */
    next(version: string): string {
        const { year, month, release } = components(version);

        return write(year, month, release + 1);
    },
});

/** A package release version. */
export type Version = z.infer<typeof Version>;

/** Write a release's calendar numbers as a version, its year in four digits. */
function write(year: number, month: number, release: number): string {
    return `${String(year).padStart(YEAR_DIGITS, "0")}.${month}.${release}`;
}

/** The versions from an oldest to a newest, both included. */
export interface VersionRange {
    /** The oldest version in the range. */
    readonly oldest: Version;
    /** The newest version in the range. */
    readonly newest: Version;
}

/** Read a version's numbers and stage, in comparison order. */
function components(version: string) {
    // read the calendar numbers and the dev mark
    const match = VERSION.exec(version);
    if (match === null) {
        throw new TypeError(`invalid version: '${version}'`);
    }
    const [, year, month, release, dev] = match;

    // sort a dev build before the release it leads to
    return {
        year: Number(year),
        month: Number(month),
        release: Number(release),
        stage: dev === undefined ? 1 : 0,
    };
}
