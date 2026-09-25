import { existsSync, readFileSync } from "node:fs";

/** The package manifest fields used by main package validation. */
type MainPackageManifest = {
    /** The package semantic version string. */
    version: string;
    /** The optional dependency versions by package name. */
    optionalDependencies?: Record<string, string>;
};

/** The platform package names expected in optionalDependencies. */
const PLATFORM_PACKAGE_NAMES = [
    "@destack/language-cli-darwin-arm64",
    "@destack/language-cli-darwin-x64",
    "@destack/language-cli-linux-arm64-gnu",
    "@destack/language-cli-linux-x64-gnu",
    "@destack/language-cli-win32-x64-msvc",
];

/** The built bin wrapper paths required for npm publish. */
const BUILT_BIN_WRAPPERS = [
    "./npm/dist/bin/destack.js",
    "./npm/dist/bin/tspp.js",
    "./npm/dist/bin/tsppc.js",
];

/** Validate the main npm package before publish. */
function validateMainPackage(): void {
    // read the package manifest
    const manifest = JSON.parse(readFileSync("package.json", "utf8")) as MainPackageManifest;
    const missingWrappers: string[] = [];
    const invalidDependencyVersions: string[] = [];

    // ensure built wrapper entrypoints exist
    for (const wrapperPath of BUILT_BIN_WRAPPERS) {
        // collect any missing wrapper paths
        if (!existsSync(wrapperPath)) {
            missingWrappers.push(wrapperPath);
        }
    }

    // ensure optional dependency versions track the main version
    for (const packageName of PLATFORM_PACKAGE_NAMES) {
        const dependencyVersion = manifest.optionalDependencies?.[packageName];

        // collect mismatched or missing dependency versions
        if (dependencyVersion !== manifest.version) {
            invalidDependencyVersions.push(`${packageName}: ${dependencyVersion ?? "missing"}`);
        }
    }

    // fail when wrappers are missing
    if (missingWrappers.length > 0) {
        console.error("error: missing built bin wrappers:");
        for (const missingWrapper of missingWrappers) {
            console.error(`  ${missingWrapper}`);
        }
        console.error("run npm run build from language/cli before publishing");
        process.exit(1);
    }

    // fail when optional dependency versions are inconsistent
    if (invalidDependencyVersions.length > 0) {
        console.error("error: optional dependency versions must match the main package version:");
        for (const invalidDependencyVersion of invalidDependencyVersions) {
            console.error(`  ${invalidDependencyVersion}`);
        }
        process.exit(1);
    }
}

validateMainPackage();
