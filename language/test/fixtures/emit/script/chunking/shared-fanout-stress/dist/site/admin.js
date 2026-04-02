import { getCommonCards } from "./routes.js";
export function getAdminModule() {
    return { heading: "admin", cards: [...getCommonCards("admin"), "admin-members"] };
}

import { getSharedInsights } from "./routes.js";
import { getPrimaryNavigation } from "./routes.js";
export function getAdminPage() {
    return {
        slug: "admin",
        navigation: getPrimaryNavigation("admin"),
        section: getAdminModule(),
        insights: getSharedInsights("admin"),
    };
}

import { renderShell } from "./routes.js";
console.log("admin", renderShell(getAdminPage()));
//# sourceMappingURL=./admin.js.map
