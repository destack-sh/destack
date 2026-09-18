import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);

/** The Linux report shape used to detect libc at runtime. */
type LinuxRuntimeReport = {
    /** The process report header metadata. */
    header?: {
        /** The detected glibc version string. */
        glibcVersionRuntime?: string;
    };
};

/** The platform package names keyed by runtime target. */
const PLATFORM_PACKAGES: Record<string, string> = {
    "darwin-arm64": "@destack/language-cli-darwin-arm64",
    "darwin-x64": "@destack/language-cli-darwin-x64",
    "linux-arm64-gnu": "@destack/language-cli-linux-arm64-gnu",
    "linux-x64-gnu": "@destack/language-cli-linux-x64-gnu",
    "win32-x64-msvc": "@destack/language-cli-win32-x64-msvc",
};

/** The supported CLI binary names exposed by npm wrappers. */
const SUPPORTED_BINARIES = new Set(["destack", "ds", "dsc"]);

/** Resolve the Linux libc family for package selection. */
function resolveLinuxLibc(): "gnu" | "musl" {
    // read the process report metadata when available
    const report = process.report?.getReport?.() as LinuxRuntimeReport | undefined;
    const glibcVersion = report?.header?.glibcVersionRuntime;

    // choose gnu when glibc metadata is present
    if (glibcVersion) {
        return "gnu";
    }

    // otherwise fall back to musl
    return "musl";
}

/** Resolve the npm platform key for the current runtime. */
function resolvePlatformKey(): string {
    // read the platform and architecture
    const platform = process.platform;
    const architecture = process.arch;

    // include libc for linux targets
    if (platform === "linux") {
        const libc = resolveLinuxLibc();
        return `${platform}-${architecture}-${libc}`;
    }

    // include toolchain for windows targets
    if (platform === "win32") {
        return `${platform}-${architecture}-msvc`;
    }

    // use platform and architecture for all other targets
    return `${platform}-${architecture}`;
}

/** Resolve the platform package name for the active runtime. */
function resolvePlatformPackageName(): string | undefined {
    const platformKey = resolvePlatformKey();

    return PLATFORM_PACKAGES[platformKey];
}

/** Resolve a locally built binary path for repository development. */
function resolveRepositoryBinaryPath(executableName: string): string | null {
    // resolve local repository paths
    const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
    const repositoryDirectory = path.resolve(scriptDirectory, "../../../..");
    const binaryPath = path.join(repositoryDirectory, "target", "release", executableName);

    // return the path when the local binary exists
    if (existsSync(binaryPath)) {
        return binaryPath;
    }

    // return null when no local binary is available
    return null;
}

/** Resolve the binary path from optional dependencies or local builds. */
function resolveBinaryPath(binaryName: string): string {
    // validate the requested binary name
    if (!SUPPORTED_BINARIES.has(binaryName)) {
        throw new Error(`unsupported binary name: ${binaryName}`);
    }

    // derive the executable name and platform package
    const executableName = process.platform === "win32" ? `${binaryName}.exe` : binaryName;
    const platformPackageName = resolvePlatformPackageName();

    // fail loudly for unsupported runtime targets
    if (!platformPackageName) {
        const platformKey = resolvePlatformKey();
        throw new Error(`unsupported platform target: ${platformKey}`);
    }

    // resolve binaries from installed platform packages
    try {
        return require.resolve(`${platformPackageName}/bin/${executableName}`);
    } catch {
        // otherwise try local development binaries
        const repositoryBinaryPath = resolveRepositoryBinaryPath(executableName);

        // return local repository binaries when available
        if (repositoryBinaryPath) {
            return repositoryBinaryPath;
        }

        // fail with installation guidance
        const platformKey = resolvePlatformKey();
        throw new Error(
            [
                `could not resolve ${platformPackageName} for ${platformKey}`,
                "run npm install -g @destack/language-cli for your platform",
                "or build locally with cargo build --release -p destack_language_cli",
            ].join("\n"),
        );
    }
}

/** Run a resolved CLI binary with inherited stdio. */
function runBinary(binaryName: string, args: readonly string[]): number {
    // resolve the binary path and spawn the command
    const binaryPath = resolveBinaryPath(binaryName);
    const launchEnvironment = { ...process.env };
    launchEnvironment.DESTACK_MANAGED_BY_NPM = "1";

    const result = spawnSync(binaryPath, args, {
        stdio: "inherit",
        env: launchEnvironment,
    });

    // surface spawn errors directly
    if (result.error) {
        throw result.error;
    }

    // return the process exit code when present
    if (typeof result.status === "number") {
        return result.status;
    }

    // propagate child process signals to the parent
    if (result.signal) {
        process.kill(process.pid, result.signal);
        return 1;
    }

    // fall back to a generic failure code
    return 1;
}

/** Run a binary command and terminate the wrapper process. */
export function runBinaryCommand(
    binaryName: string,
    args: readonly string[] = process.argv.slice(2),
): never {
    // run the command and exit with its status code
    try {
        const exitCode = runBinary(binaryName, args);
        process.exit(exitCode);
    } catch (error) {
        // print explicit errors and fail loudly
        if (error instanceof Error) {
            console.error(`error: ${error.message}`);
        }
        // report unknown thrown values
        else {
            console.error("error: failed to launch destack binary");
        }
        process.exit(1);
    }
}
