import { satisfies } from "semver";
import * as z from "zod";

/** A version: dev, or a release's calendar version such as 2026.9.0. */
const VERSION = /^(?:dev|(\d{4})\.([1-9]|1[0-2])\.(0|[1-9]\d*))$/u;

/** The digits of a version's year. */
const YEAR_DIGITS = 4;

/** The version of unreleased code, as Go's (devel). */
const DEVELOPMENT = "dev";

/** The schema of a package version, which the version's verbs write through. */
const versionSchema = z.string().regex(VERSION);

/** A package version: a release such as 2026.9.0, or dev for unreleased code. */
export const Version = Object.assign(versionSchema, {
    /** The version of unreleased code, which orders after every release. */
    development: DEVELOPMENT,

    /** Report whether a version is a release. */
    isRelease(version: string): boolean {
        return version !== DEVELOPMENT;
    },

    /** Report whether a version meets a dependency range, unreleased code meeting any. */
    meets(version: string, range: string): boolean {
        return version === DEVELOPMENT || (range !== DEVELOPMENT && satisfies(version, range));
    },

    /** Order versions by release, unreleased code after every release. */
    compare(left: string, right: string): number {
        // order unreleased code after every release
        if (left === DEVELOPMENT || right === DEVELOPMENT) {
            return Number(left === DEVELOPMENT) - Number(right === DEVELOPMENT);
        }

        // find the first differing calendar number
        const first = components(left);
        const second = components(right);
        const differences = [
            first.year - second.year,
            first.month - second.month,
            first.release - second.release,
        ];

        return Math.sign(differences.find((difference) => difference !== 0) ?? 0);
    },

    /** Write the first release of the UTC month a time lies in. */
    first(at: number): string {
        const time = new Date(at);

        return `${time.getUTCFullYear()}.${time.getUTCMonth() + 1}.0`;
    },

    /** Read the next release in a release's year and month. */
    next(version: string): string {
        const { year, month, release } = components(version);

        return write(year, month, release + 1);
    },
});

/** A package version. */
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

/** Read a release's calendar numbers. */
function components(version: string) {
    // read the calendar numbers
    const [, year, month, release] = VERSION.exec(version) ?? [];
    if (year === undefined) {
        throw new TypeError(`version '${version}' is not a release`);
    }

    return { year: Number(year), month: Number(month), release: Number(release) };
}
