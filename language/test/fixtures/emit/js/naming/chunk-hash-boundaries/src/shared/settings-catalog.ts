import { getSharedLabel } from "./shared-label.ts";

export function getSettingsCatalog() {
    return [
        getSharedLabel("alpha"),
        getSharedLabel("beta"),
        getSharedLabel("settings"),
    ];
}
