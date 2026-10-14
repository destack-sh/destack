import { ObjectExtension } from "@destack/object/server";
import settingPackage from "../package.ts";
import { implementSetting, type SettingOptions } from "./setting.ts";

/** Implement the setting package in a store, checking written values against their declarations. */
export function settingExtension(options: SettingOptions): ObjectExtension {
    return ObjectExtension.of(settingPackage, [implementSetting(options)]);
}
