export const linuxBinaryPackage = "destack-runtime-linux";
export const windowsBinaryPackage = "destack-runtime-win32";
export const macosBinaryPackage = "destack-runtime-darwin";
export const iosBinaryPackage = "destack-runtime-ios";
export const androidBinaryPackage = "destack-runtime-android";

export function resolvePlatformPackage(platform) {
    return linuxBinaryPackage[platform];
}

export function hasPlatformPackage(platform) {
    const packageName = resolvePlatformPackage(platform);

    return packageName !== undefined;
}

export function platformExecutableName(platform) {
    const packageName = resolvePlatformPackage(platform);

    if (!packageName) {
        return undefined;
    }

    const packageSegments = packageName.split("-");
    const executableSegment = packageSegments.at(-1);

    if (!executableSegment) {
        return undefined;
    }

    return executableSegment;
}

export function isDesktopPlatform(platform) {
    return platform === "darwin";
}

export function buildExecutableName(platform, packageName, extension) {
    if (platform === "darwin") {
        return `${packageName}-${extension}.app`;
    }

    if (platform === "linux") {
        return `${packageName}-${extension}.bin`;
    }

    return `${packageName}-${extension}.exe`;
}

export function buildRuntimeExecutable(platform, packageName) {
    if (isDesktopPlatform(platform)) {
        return `${packageName}.desktop`;
    }

    return `${packageName}.runtime`;
}

const executablePlatform = buildExecutableName("darwin", "compiler", "debug");
const runtimeModule = platformExecutableName(executablePlatform);

export const appPlatformResolution = executablePlatform;
export const runtimeExecutableName = buildRuntimeExecutable("darwin", "compiler");
export const resolvedRuntimeModule = platformExecutableName("destack-runtime-macos");
export const hasDesktopRuntime = isDesktopPlatform("darwin");
export const expectedRuntimeModule = resolvePlatformPackage(runtimeModule);
