import { Commit, schema, Version } from "@destack/schema";
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
export const Target = schema.enum(TARGETS);

/** A supported distribution target. */
export type Target = schema.Infer<typeof Target>;

/** The custom field of a signed TUF target: the release it distributes. */
export const TargetCustom = schema
    .object({
        /** The calendar version the executables report. */
        version: Version,
        /** The Git commit the distribution was built from. */
        commit: Commit,
    })
    .strip();

/** The update channels a release publishes on, each with its own signed repository. */
export const CHANNELS = ["stable", "nightly"] as const;

/** An update channel. */
export type Channel = (typeof CHANNELS)[number];

/** The release architecture of each Node architecture name. */
const ARCHITECTURES: Readonly<Record<string, string | undefined>> = {
    arm64: "aarch64",
    x64: "x86_64",
};

/** The release system of each Node platform name. */
const SYSTEMS: Readonly<Record<string, string | undefined>> = {
    darwin: "apple-darwin",
    linux: "unknown-linux-gnu",
    win32: "pc-windows-msvc",
};

/** A calendar release identified by authenticated update metadata. */
export class Release {
    /** Identify the release target of a platform and architecture, as Node names them in `process.platform` and `process.arch`. */
    static target(platform: string, architecture: string): Target {
        // map the platform and architecture to a release target
        const target = Target.safeParse(`${ARCHITECTURES[architecture]}-${SYSTEMS[platform]}`);
        if (!target.success) {
            throw new UpdateError("RELEASE", `unsupported target: ${platform}/${architecture}`);
        }

        return target.data;
    }

    /** Calendar version of the distribution. */
    readonly version: string;
    /** Operating system and architecture of the distribution. */
    readonly target: Target;

    /** Validate the version and target from signed metadata. */
    constructor(version: unknown, target: string) {
        // validate the public release identity before selecting its platform
        const parsedVersion = Version.safeParse(version);
        if (!parsedVersion.success) {
            throw new UpdateError("RELEASE", "invalid release version");
        }
        const parsedTarget = Target.safeParse(target);
        if (!parsedTarget.success) {
            throw new UpdateError("RELEASE", `unsupported target: ${target}`);
        }
        this.version = parsedVersion.data;
        this.target = parsedTarget.data;
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

    /** Update channel selected by this release identity. */
    get channel(): Channel {
        return this.version.includes("-nightly.") ? "nightly" : "stable";
    }
}
