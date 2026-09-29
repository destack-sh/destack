import { Version } from "@destack/schema";
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
        if (!Version.safeParse(version).success) {
            throw new UpdateError("RELEASE", "invalid release version");
        }
        if (!TARGETS.includes(target as Target)) {
            throw new UpdateError("RELEASE", `unsupported target: ${target}`);
        }
        this.version = version as string;
        this.target = target as Target;
        Object.freeze(this);
    }

    /** Compare calendar versions in release order. */
    compare(other: Release): number {
        return Version.compare(this.version, other.version);
    }

    /** Name the immutable installed distribution directory. */
    get directory(): string {
        return `${this.version}-${this.target}`;
    }

    /** Update feed selected by this release identity. */
    get channel(): "stable" | "nightly" {
        return this.version.includes("-nightly.") ? "nightly" : "stable";
    }
}
