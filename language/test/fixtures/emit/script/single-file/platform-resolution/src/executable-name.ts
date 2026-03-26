/** The Windows platform identifier. */
export const windowsPlatform = "win32";

/** Check whether a platform key is the Windows platform. */
export function isWindowsPlatform(platform: string) {
    return platform === windowsPlatform;
}

/** Build the executable file name for a binary on one platform. */
export function getExecutableName(binaryName: string, isWindows: boolean) {
    if (isWindows) {
        return `${binaryName}.exe`;
    }

    return binaryName;
}

/** Build a binary label for one platform specific executable. */
export function getBinaryLabel(binaryName: string, platform: string) {
    const isWindows = isWindowsPlatform(platform);
    const executableName = getExecutableName(binaryName, isWindows);

    return `${platform}:${executableName}`;
}
