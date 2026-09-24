import { UpdateError } from "../error/error.ts";

/** Calendar version with an optional nightly build sequence. */
const VERSION = /^(\d{4})\.([1-9]|1[0-2])\.(0|[1-9]\d*)(?:-nightly\.(0|[1-9]\d*))?$/;

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
    /** Identify the running distribution's operating system and architecture. */
    static target(): Target {
        // map the process architecture and platform to a release target
        const architecture =
            process.arch === "arm64" ? "aarch64" : process.arch === "x64" ? "x86_64" : undefined;
        const system =
            process.platform === "darwin"
                ? "apple-darwin"
                : process.platform === "linux"
                  ? "unknown-linux-gnu"
                  : process.platform === "win32"
                    ? "pc-windows-msvc"
                    : undefined;
        const target = `${architecture}-${system}`;
        if (!TARGETS.includes(target as Target)) {
            throw new UpdateError(
                "RELEASE",
                `unsupported target: ${process.platform}/${process.arch}`,
            );
        }

        return target as Target;
    }

    /** Calendar version of the distribution. */
    readonly version: string;
    /** Operating system and architecture of the distribution. */
    readonly target: Target;

    /** Validate the version and target from signed metadata. */
    constructor(version: unknown, target: string) {
        // validate the public release identity before selecting its platform
        parseVersion(version);
        if (!TARGETS.includes(target as Target)) {
            throw new UpdateError("RELEASE", `unsupported target: ${target}`);
        }
        this.version = version as string;
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

    /** Update feed selected by this release identity. */
    get channel(): "stable" | "nightly" {
        return this.version.includes("-nightly.") ? "nightly" : "stable";
    }

    /** Native application identifier for this release. */
    get applicationIdentifier(): string {
        return this.channel === "stable" ? "sh.destack.desktop" : "sh.destack.desktop.nightly";
    }
}

/** Compare validated calendar versions independently of their distribution format. */
export function compareVersions(left: string, right: string): number {
    // compare calendar components before the optional prerelease sequence
    const first = parseVersion(left);
    const second = parseVersion(right);
    for (let index = 0; index < first.length; index++) {
        if (first[index] !== second[index]) {
            return Math.sign(first[index] - second[index]);
        }
    }

    return 0;
}

/** Read numeric calendar components and order stable after its nightly prereleases. */
function parseVersion(version: unknown): number[] {
    // reject malformed identities before filesystem or ordering operations
    const match = typeof version === "string" ? VERSION.exec(version) : null;
    if (!match) {
        throw new UpdateError("RELEASE", "invalid release version");
    }
    const components = match.slice(1, 4).map(Number);
    const sequence = match[4] === undefined ? undefined : Number(match[4]);
    if (
        !components.every(Number.isSafeInteger) ||
        (sequence !== undefined && !Number.isSafeInteger(sequence))
    ) {
        throw new UpdateError("RELEASE", "invalid release version");
    }

    return [...components, sequence === undefined ? 1 : 0, sequence === undefined ? 0 : sequence];
}
