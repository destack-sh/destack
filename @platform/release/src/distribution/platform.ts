import { schema } from "@destack/schema";
import type { Target } from "@destack/update/release";

/** The native installer formats a release publishes beside its update archives. */
export const INSTALLER_FORMATS = ["dmg"] as const;

/** The code signers of the platforms that sign their applications. */
export const SIGNERS = ["apple"] as const;

/** The macOS application bundle Tauri builds. */
export const APPLE_BUNDLE: Bundle = {
    tauri: "app",
    application: "Destack.app",
    executable: "Contents/MacOS/Destack",
    helpers: "Contents/Helpers",
};

/** The Linux application directory around the bare native executable. */
const LINUX_BUNDLE: Bundle = {
    tauri: undefined,
    application: "Destack",
    executable: "Destack",
    helpers: "helpers",
};

/** The universal macOS disk image of both macOS targets. */
const UNIVERSAL_DMG: Installer = {
    format: "dmg",
    name: "universal-apple-darwin",
    runner: "macos-15",
};

/** The released targets, each with how it builds, signs and installs. */
export const PLATFORMS: readonly Platform[] = [
    {
        target: "aarch64-apple-darwin",
        runner: "macos-15",
        runtime: "bun-darwin-arm64",
        system: "darwin",
        architecture: "arm64",
        bundle: APPLE_BUNDLE,
        signer: "apple",
        installers: [UNIVERSAL_DMG],
    },
    {
        target: "x86_64-apple-darwin",
        runner: "macos-15-intel",
        runtime: "bun-darwin-x64",
        system: "darwin",
        architecture: "x64",
        bundle: APPLE_BUNDLE,
        signer: "apple",
        installers: [UNIVERSAL_DMG],
    },
    {
        target: "aarch64-unknown-linux-gnu",
        runner: "ubuntu-24.04-arm",
        runtime: "bun-linux-arm64",
        system: "linux",
        architecture: "arm64",
        bundle: LINUX_BUNDLE,
        signer: undefined,
        installers: [],
    },
    {
        target: "x86_64-unknown-linux-gnu",
        runner: "ubuntu-24.04",
        runtime: "bun-linux-x64",
        system: "linux",
        architecture: "x64",
        bundle: LINUX_BUNDLE,
        signer: undefined,
        installers: [],
    },
];

/** A native installer format. */
export const InstallerFormat = schema.enum(INSTALLER_FORMATS);

/** A native installer format. */
export type InstallerFormat = schema.Infer<typeof InstallerFormat>;

/** A platform code signer. */
export type SignerName = (typeof SIGNERS)[number];

/** One released target: its runner, Bun runtime, native desktop bundle, signer and installers. */
export interface Platform {
    /** The release target. */
    readonly target: Target;
    /** The GitHub runner building, signing and verifying the target. */
    readonly runner: string;
    /** The Bun runtime compiled into the target's executables. */
    readonly runtime: Bun.Build.CompileTarget;
    /** The operating system, as Node names it in `process.platform`. */
    readonly system: NodeJS.Platform;
    /** The architecture, as Node names it in `process.arch`. */
    readonly architecture: NodeJS.Architecture;
    /** The layout of the native desktop in the distribution. */
    readonly bundle: Bundle;
    /** The code signer of the application, or none when TUF and digests alone authenticate it. */
    readonly signer: SignerName | undefined;
    /** The native installers the target's application ships in. */
    readonly installers: readonly Installer[];
}

/** The layout of a native desktop in a distribution. */
export interface Bundle {
    /** The Tauri bundle the native desktop builds as, or none for the bare executable. */
    readonly tauri: "app" | undefined;
    /** The application directory in the distribution. */
    readonly application: string;
    /** The native desktop executable, relative to the application directory. */
    readonly executable: string;
    /** The directory of the commands the native desktop starts, relative to the application directory. */
    readonly helpers: string;
}

/** A native installer built from the applications of every target that lists it. */
export interface Installer {
    /** The file format of the installer. */
    readonly format: InstallerFormat;
    /** The download name the installer publishes under in place of a target. */
    readonly name: string;
    /** The GitHub runner building the installer. */
    readonly runner: string;
}

/** An installer with the platforms whose applications it ships. */
export interface InstallerPlatforms {
    /** The installer. */
    readonly installer: Installer;
    /** The platforms whose applications the installer ships. */
    readonly platforms: readonly Platform[];
}

/** Find the platform of a released target. */
export function selectPlatform(target: Target): Platform {
    // refuse a target without a platform row
    const platform = PLATFORMS.find((candidate) => candidate.target === target);
    if (platform === undefined) {
        throw new Error(`unsupported distribution target: ${target}`);
    }

    return platform;
}

/** List each installer of the released platforms with the platforms whose applications it ships. */
export function listInstallers(): readonly InstallerPlatforms[] {
    // group the platforms by the installer rows they share
    const installers = new Set(PLATFORMS.flatMap((platform) => platform.installers));

    return [...installers].map((installer) => ({
        installer,
        platforms: PLATFORMS.filter((platform) => platform.installers.includes(installer)),
    }));
}
