import { Commit, present, schema, Version } from "@destack/schema";
import type { OperatingSystem } from "@destack/environment";
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

/** The release architecture of each Node architecture name. */
const ARCHITECTURES: Readonly<Record<string, string | undefined>> = {
    arm64: "aarch64",
    x64: "x86_64",
};

/** The processes a release's executable runs besides its command line, by their package. */
export const PROCESSES = ["machine", "build", "sandbox", "desktop"] as const;

/** A process a release's executable runs besides its command line. */
export type ReleaseProcess = (typeof PROCESSES)[number];

/** The option selecting the process a release's executable runs. */
const PROCESS_OPTION = "--process=";

/** The release system of each Node platform name. */
const SYSTEMS: Readonly<Record<string, string | undefined>> = {
    darwin: "apple-darwin",
    linux: "unknown-linux-gnu",
    win32: "pc-windows-msvc",
};

/** A calendar release identified by authenticated update metadata. */
export class Release {
    /** Identify the release target of a platform and architecture. */
    static target(platform: string, architecture: string): Target {
        // map the platform and architecture to a release target
        const target = Target.safeParse(`${ARCHITECTURES[architecture]}-${SYSTEMS[platform]}`);
        if (!target.success) {
            throw UpdateError.of(
                "TARGET_INVALID",
                `target '${platform}/${architecture}' is invalid: it is not supported`,
            );
        }

        return target.data;
    }

    /** Read the file name of a release's executable, with `.exe` on Windows. */
    static executable(os: OperatingSystem): string {
        return os === "win32" ? "destack.exe" : "destack";
    }

    /** Write the arguments starting a process of a release's executable. */
    static arguments(process: ReleaseProcess): string[] {
        return [`${PROCESS_OPTION}${process}`];
    }

    /** Read the process a release's executable starts with, absent for its command line. */
    static process(words: readonly string[]): ReleaseProcess | undefined {
        // take the process option as the first word alone
        const [first = ""] = words;
        if (!first.startsWith(PROCESS_OPTION)) {
            return undefined;
        }
        const name = first.slice(PROCESS_OPTION.length);

        // refuse a process the release does not run
        const process = PROCESSES.find((each) => each === name);
        if (process === undefined) {
            throw UpdateError.of("PROCESS_INVALID", `process '${name}' is invalid: it is unknown`);
        }

        return process;
    }

    /** Read the operating system a release target runs on. */
    static system(target: Target): OperatingSystem {
        const systems: readonly OperatingSystem[] = ["darwin", "linux", "win32"];

        return present(
            systems.find((os) => target.endsWith(`-${SYSTEMS[os]}`)),
            `the operating system of ${target}`,
        );
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
            throw UpdateError.of("RELEASE_BROKEN", "could not read the release version");
        }
        const parsedTarget = Target.safeParse(target);
        if (!parsedTarget.success) {
            throw UpdateError.of(
                "TARGET_INVALID",
                `target '${target}' is invalid: it is not supported`,
            );
        }
        this.version = parsedVersion.data;
        this.target = parsedTarget.data;
        Object.freeze(this);
    }

    /** Compare calendar versions in release order. */
    compare(other: Release): number {
        return Version.compare(this.version, other.version);
    }

    /** Build the path of the immutable installed distribution directory. */
    get directory(): string {
        return `${this.version}-${this.target}`;
    }
}
