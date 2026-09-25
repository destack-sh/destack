import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

/** The package manifest fields used by platform package validation. */
type PlatformPackageManifest = {
    /** The package semantic version string. */
    version: string;
    /** The supported operating systems for the package. */
    os?: string[];
};

/** The package manifest fields used by main package version checks. */
type MainPackageManifest = {
    /** The package semantic version string. */
    version: string;
};

/** The CLI binary names expected in each platform package. */
const BINARY_NAMES = ["tspp", "tsppc"];

/** Validate a platform npm package before publish. */
function validatePlatformPackage(): void {
    // read package metadata
    const manifest = JSON.parse(readFileSync("package.json", "utf8")) as PlatformPackageManifest;
    const mainManifestPath = path.join("..", "package.json");
    const isWindows = manifest.os?.includes("win32") ?? false;
    const executableSuffix = isWindows ? ".exe" : "";
    const missingBinaries: string[] = [];

    // ensure the main package metadata is available
    if (!existsSync(mainManifestPath)) {
        console.error(`error: missing main package manifest: ${mainManifestPath}`);
        process.exit(1);
    }

    // ensure platform package versions match the main package version
    const mainManifest = JSON.parse(readFileSync(mainManifestPath, "utf8")) as MainPackageManifest;
    if (manifest.version !== mainManifest.version) {
        console.error(
            `error: platform package version ${manifest.version} does not match main package version ${mainManifest.version}`,
        );
        process.exit(1);
    }

    // ensure every binary is present in the package bin directory
    for (const binaryName of BINARY_NAMES) {
        const executableName = `${binaryName}${executableSuffix}`;
        const binaryPath = path.join("bin", executableName);

        // collect binaries that have not been staged yet
        if (!existsSync(binaryPath)) {
            missingBinaries.push(binaryPath);
        }
    }

    // fail when binaries have not been staged yet
    if (missingBinaries.length > 0) {
        console.error("error: missing platform binaries:");
        for (const missingBinary of missingBinaries) {
            console.error(`  ${missingBinary}`);
        }
        console.error("run npm run stage:binaries from language/cli before publishing");
        process.exit(1);
    }
}

validatePlatformPackage();
