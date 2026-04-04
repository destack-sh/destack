import { getSharedInsights } from "../shared/data/insights.ts";
import { getPrimaryNavigation } from "../shared/navigation.ts";
import { getAdminModule } from "../shared/sections/admin.ts";

export function getAdminPage() {
    return {
        slug: "admin",
        navigation: getPrimaryNavigation("admin"),
        section: getAdminModule(),
        insights: getSharedInsights("admin"),
    };
}
