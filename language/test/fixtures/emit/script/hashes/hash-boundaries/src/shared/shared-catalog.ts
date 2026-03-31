import { getSharedLabel } from "./shared-label.ts";

export function getSharedCatalog() {
    return [
        getSharedLabel("alpha"),
        getSharedLabel("beta"),
        getSharedLabel("gamma"),
    ];
}
