import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** The supported npm platform package targets. */
type PlatformTarget = {
    /** The package suffix used in npm package names. */
    packageSuffix: string;
    /** The Rust target triple used for binary builds. */
    targetTriple: string;
    /** The target is windows and requires exe binaries. */
    isWindows: boolean;
};

/** The platform package targets used for npm distribution. */
const PLATFORM_TARGETS: PlatformTarget[] = [
    {
        packageSuffix: "darwin-arm64",
        targetTriple: "aarch64-apple-darwin",
        isWindows: false,
    },
    {
        packageSuffix: "darwin-x64",
        targetTriple: "x86_64-apple-darwin",
        isWindows: false,
    },
    {
        packageSuffix: "linux-arm64-gnu",
        targetTriple: "aarch64-unknown-linux-gnu",
        isWindows: false,
    },
    {
        packageSuffix: "linux-x64-gnu",
        targetTriple: "x86_64-unknown-linux-gnu",
        isWindows: false,
    },
    {
        packageSuffix: "win32-x64-msvc",
        targetTriple: "x86_64-pc-windows-msvc",
        isWindows: true,
    },
];

/** The CLI binary names expected in each target package. */
const BINARY_NAMES = ["tspp", "tsppc"];

/** Resolve platform targets from TSPP_RELEASE_TARGETS when provided. */
function resolveActivePlatformTargets(): PlatformTarget[] {
    // read the optional target filter from the environment
    const targetsInput = process.env.TSPP_RELEASE_TARGETS;
    if (!targetsInput) {
        return PLATFORM_TARGETS;
    }

    // split target tokens on whitespace
    const targetTokens = targetsInput
        .split(/\s+/)
        .map((token) => token.trim())
        .filter((token) => token.length > 0);
    const activeTargets: PlatformTarget[] = [];
    const unknownTokens: string[] = [];

    // map each token to a known platform target
    for (const targetToken of targetTokens) {
        const platformTarget = PLATFORM_TARGETS.find(
            (candidateTarget) =>
                candidateTarget.targetTriple === targetToken ||
                candidateTarget.packageSuffix === targetToken,
        );

        // keep track of unknown filter values
        if (!platformTarget) {
            unknownTokens.push(targetToken);
            continue;
        }

        // avoid duplicate targets when aliases overlap
        const alreadySelected = activeTargets.some(
            (selectedTarget) => selectedTarget.targetTriple === platformTarget.targetTriple,
        );
        if (!alreadySelected) {
            activeTargets.push(platformTarget);
        }
    }

    // fail loudly when unknown target filters are provided
    if (unknownTokens.length > 0) {
        console.error("error: unsupported TSPP_RELEASE_TARGETS entries:");
        for (const unknownToken of unknownTokens) {
            console.error(`  ${unknownToken}`);
        }
        process.exit(1);
    }

    // fail loudly when filtering resolved to no targets
    if (activeTargets.length === 0) {
        console.error("error: TSPP_RELEASE_TARGETS did not resolve to any npm platform targets");
        process.exit(1);
    }

    return activeTargets;
}

/** Stage compiled release binaries into npm platform package directories. */
function stagePlatformBinaries(): void {
    // resolve local repository paths
    const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
    const cliDirectory = path.resolve(scriptDirectory, "..");
    const repositoryDirectory = path.resolve(cliDirectory, "../..");
    const activeTargets = resolveActivePlatformTargets();
    const missingPaths: string[] = [];

    // copy binaries from target triples into platform packages
    for (const platformTarget of activeTargets) {
        const sourceDirectory = path.join(
            repositoryDirectory,
            "target",
            platformTarget.targetTriple,
            "release",
        );
        const destinationDirectory = path.join(
            cliDirectory,
            `npm-${platformTarget.packageSuffix}`,
            "bin",
        );

        mkdirSync(destinationDirectory, { recursive: true });

        for (const binaryName of BINARY_NAMES) {
            const executableName = platformTarget.isWindows ? `${binaryName}.exe` : binaryName;
            const sourcePath = path.join(sourceDirectory, executableName);
            const destinationPath = path.join(destinationDirectory, executableName);

            // keep track of missing source binaries for a loud failure
            if (!existsSync(sourcePath)) {
                missingPaths.push(sourcePath);
                continue;
            }

            // copy built binaries into the platform package
            copyFileSync(sourcePath, destinationPath);

            // set executable permissions for unix targets
            if (!platformTarget.isWindows) {
                chmodSync(destinationPath, 0o755);
            }
        }
    }

    // fail loudly when source binaries are missing
    if (missingPaths.length > 0) {
        console.error("error: missing release binaries:");
        for (const missingPath of missingPaths) {
            console.error(`  ${missingPath}`);
        }
        process.exit(1);
    }

    // report success
    console.log("staged cli binaries into npm platform package directories");
}

stagePlatformBinaries();
