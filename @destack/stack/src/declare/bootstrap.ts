import objectPackage from "@destack/object/package";
import settingPackage from "@destack/setting/package";

/** The packages a space's installer needs, which the space serves as its own, by name. */
export const BOOTSTRAP_PACKAGES = {
    stack: import.meta.destack.package,
    object: objectPackage,
    setting: settingPackage,
} as const;

/** The name of one of the packages a space's installer needs. */
export type BootstrapPackageName = keyof typeof BOOTSTRAP_PACKAGES;
