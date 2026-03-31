/** The platform package names keyed by runtime target. */
export const platformPackages = {
    "darwin-arm64": "@destack/cli-darwin-arm64",
    "darwin-x64": "@destack/cli-darwin-x64",
    "linux-arm64-gnu": "@destack/cli-linux-arm64-gnu",
    "linux-x64-gnu": "@destack/cli-linux-x64-gnu",
    "win32-x64-msvc": "@destack/cli-win32-x64-msvc",
};

/** Resolve the package name for one platform key. */
export function resolvePlatformPackageName(platformKey: string) {
    return platformPackages[platformKey];
}

/** Check whether a platform package exists for one platform key. */
export function isPlatformPackageAvailable(platformKey: string) {
    const packageName = getPlatformPackageSegment(platformKey);

    return packageName !== "missing";
}

/** Read the package segment from a scoped platform package name. */
export function getPlatformPackageSegment(platformKey: string) {
    const packageName = resolvePlatformPackageName(platformKey);

    // fall back when no package is defined
    if (!packageName) {
        return "missing";
    }

    const segments = packageName.split("/");
    const packageSegment = segments[segments.length - 1];

    // fall back when the scoped package has no segment
    if (!packageSegment) {
        return "missing";
    }

    return packageSegment;
}
