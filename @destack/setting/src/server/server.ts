import { implementPackage, type PackageImplementation } from "@destack/object/server";
import settingPackage from "../package.ts";
import { implementSetting, type SettingOptions } from "./setting.ts";

/** Serve the setting package, checking written values against their declarations. */
export function settingServer(options: SettingOptions): PackageImplementation {
    return implementPackage(settingPackage, { objects: [implementSetting(options)] });
}
