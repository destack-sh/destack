import { defineServer, type PackageServer } from "@destack/object/server";
import settingPackage from "../package.ts";
import { implementSetting, type SettingOptions } from "./setting.ts";

/** Serve the setting package, checking written values against their declarations. */
export function settingServer(options: SettingOptions): PackageServer {
    return defineServer(settingPackage, { objects: [implementSetting(options)] });
}
