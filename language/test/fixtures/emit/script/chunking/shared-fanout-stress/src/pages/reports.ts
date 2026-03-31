import { getSharedInsights } from "../shared/data/insights.ts";
import { getPrimaryNavigation } from "../shared/navigation.ts";
import { getReportsModule } from "../shared/sections/reports.ts";

export function getReportsPage() {
    return {
        slug: "reports",
        navigation: getPrimaryNavigation("reports"),
        section: getReportsModule(),
        insights: getSharedInsights("reports"),
    };
}
