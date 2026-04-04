import { getSharedInsights } from "../shared/data/insights.ts";
import { getPrimaryNavigation } from "../shared/navigation.ts";
import { getOverviewModule } from "../shared/sections/overview.ts";

export function getOverviewPage() {
    return {
        slug: "overview",
        navigation: getPrimaryNavigation("overview"),
        section: getOverviewModule(),
        insights: getSharedInsights("overview"),
    };
}
