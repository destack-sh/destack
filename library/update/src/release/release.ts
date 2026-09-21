import { UpdateError } from "../error/error.ts";

/** Operating systems and architectures supported by Destack distributions. */
export const TARGETS = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
] as const;

/** A supported distribution target. */
export type Target = (typeof TARGETS)[number];

/** A calendar release identified by authenticated update metadata. */
export class Release {
    /** Calendar version of the distribution. */
    readonly version: string;
    /** Operating system and architecture of the distribution. */
    readonly target: Target;

    /** Validate the version and target from signed metadata. */
    constructor(version: unknown, target: string) {
        if (
            typeof version !== "string" ||
            !/^\d{4}\.(?:[1-9]|1[0-2])\.(?:0|[1-9]\d*)$/.test(version)
        ) {
            throw new UpdateError("RELEASE", "Invalid release version.");
        }
        if (!TARGETS.includes(target as Target)) {
            throw new UpdateError("RELEASE", `Unsupported target: ${target}`);
        }
        if (!version.split(".").every((value) => Number.isSafeInteger(Number(value)))) {
            throw new UpdateError("RELEASE", "Invalid release version.");
        }
        this.version = version;
        this.target = target as Target;
        Object.freeze(this);
    }

    /** Compare calendar versions in release order. */
    compare(other: Release): number {
        return compareVersions(this.version, other.version);
    }

    /** Name the immutable installed distribution directory. */
    get directory(): string {
        return `${this.version}-${this.target}`;
    }
}

/** Compare validated calendar versions independently of their distribution format. */
export function compareVersions(left: string, right: string): number {
    for (const version of [left, right]) {
        if (
            !/^\d{4}\.(?:[1-9]|1[0-2])\.(?:0|[1-9]\d*)$/.test(version) ||
            !version.split(".").every((value) => Number.isSafeInteger(Number(value)))
        ) {
            throw new UpdateError("RELEASE", "Invalid release version.");
        }
    }
    const first = left.split(".").map(Number);
    const second = right.split(".").map(Number);
    for (let index = 0; index < first.length; index++) {
        if (first[index] !== second[index]) {
            return Math.sign(first[index] - second[index]);
        }
    }

    return 0;
}
