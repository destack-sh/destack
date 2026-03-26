/** Check whether a platform is the Linux platform. */
export function isLinuxPlatform(platform: string) {
    return platform === "linux";
}

/** Resolve the package lookup key for one runtime target. */
export function resolvePlatformKey(
    platform: string,
    architecture: string,
    libc: string,
) {
    if (platform === "linux") {
        return `${platform}-${architecture}-${libc}`;
    }

    if (platform === "win32") {
        return `${platform}-${architecture}-msvc`;
    }

    return `${platform}-${architecture}`;
}

/** Build the runtime label for one runtime target. */
export function getRuntimeLabel(platform: string, architecture: string) {
    if (isLinuxPlatform(platform)) {
        return `${platform}/${architecture}/server`;
    }
    return `${platform}/${architecture}/desktop`;
}
