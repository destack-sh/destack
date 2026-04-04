import { getCommonCards } from "./chunk-2a084f36.js";
export function getAdminModule() {
    return { heading: "admin", cards: [...getCommonCards("admin"), "admin-members"] };
}

import { getSharedInsights } from "./chunk-2a084f36.js";
import { getPrimaryNavigation } from "./chunk-2a084f36.js";
export function getAdminPage() {
    return {
        slug: "admin",
        navigation: getPrimaryNavigation("admin"),
        section: getAdminModule(),
        insights: getSharedInsights("admin"),
    };
}

import { renderShell } from "./chunk-2a084f36.js";
console.log("admin", renderShell(getAdminPage()));
//# sourceMappingURL=./admin.js.map
