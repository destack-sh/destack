import { getBinaryLabel, getExecutableName } from "./executable-name.ts";
import {
    getPlatformPackageSegment,
    isPlatformPackageAvailable,
    resolvePlatformPackageName,
} from "./platform-packages.ts";
import { getRuntimeLabel, resolvePlatformKey } from "./resolve-platform.ts";

const platformKey = resolvePlatformKey("linux", "x64", "gnu");
const packageName = resolvePlatformPackageName(platformKey);
const executableName = getExecutableName("destack", false);
const binaryLabel = getBinaryLabel("destack", "linux");
const runtimeLabel = getRuntimeLabel("linux", "x64");
const packageSegment = getPlatformPackageSegment(platformKey);

/** The platform resolution state for the single file entry. */
export const appPlatformResolution = {
    platformKey,
    packageName,
    executableName,
    binaryLabel,
    runtimeLabel,
    hasPlatformPackage: isPlatformPackageAvailable(platformKey),
    packageSegment,
};
