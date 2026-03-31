export const platformPackages = {
    "darwin-arm64": "@destack/cli-darwin-arm64",
    "darwin-x64": "@destack/cli-darwin-x64",
    "linux-arm64-gnu": "@destack/cli-linux-arm64-gnu",
    "linux-x64-gnu": "@destack/cli-linux-x64-gnu",
    "win32-x64-msvc": "@destack/cli-win32-x64-msvc",
};
export function resolvePlatformPackageName(platformKey) {
    return platformPackages[platformKey];
}
export function isPlatformPackageAvailable(platformKey) {
    const packageName = getPlatformPackageSegment(platformKey);
    return packageName !== "missing";
}
export function getPlatformPackageSegment(platformKey) {
    const packageName = resolvePlatformPackageName(platformKey);
    if(!packageName) {
        return "missing";
    }
    const segments = packageName.split("/");
    const packageSegment = segments[segments.length - 1];
    if(!packageSegment) {
        return "missing";
    }
    return packageSegment;
}

export function isLinuxPlatform(platform) {
    return platform === "linux";
}
export function resolvePlatformKey(platform, architecture, libc) {
    if(platform === "linux") {
        return `${platform}-${architecture}-${libc}`;
    }
    if(platform === "win32") {
        return `${platform}-${architecture}-msvc`;
    }
    return `${platform}-${architecture}`;
}
export function getRuntimeLabel(platform, architecture) {
    if(isLinuxPlatform(platform)) {
        return `${platform}/${architecture}/server`;
    }
    return `${platform}/${architecture}/desktop`;
}

export const windowsPlatform = "win32";
export function isWindowsPlatform(platform) {
    return platform === windowsPlatform;
}
export function getExecutableName(binaryName, isWindows) {
    if(isWindows) {
        return `${binaryName}.exe`;
    }
    return binaryName;
}
export function getBinaryLabel(binaryName, platform) {
    const isWindows = isWindowsPlatform(platform);
    const executableName = getExecutableName(binaryName, isWindows);
    return `${platform}:${executableName}`;
}

const platformKey = resolvePlatformKey("linux", "x64", "gnu");
const packageName = resolvePlatformPackageName(platformKey);
const executableName = getExecutableName("destack", false);
const binaryLabel = getBinaryLabel("destack", "linux");
const runtimeLabel = getRuntimeLabel("linux", "x64");
const packageSegment = getPlatformPackageSegment(platformKey);
export const appPlatformResolution = {
    platformKey,
    packageName,
    executableName,
    binaryLabel,
    runtimeLabel,
    hasPlatformPackage: isPlatformPackageAvailable(platformKey),
    packageSegment,
};
//# sourceMappingURL=./app.js.map
