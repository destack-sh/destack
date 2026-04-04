import { getSharedInsights } from "../shared/data/insights.ts";
import { getPrimaryNavigation } from "../shared/navigation.ts";
import { getDocsModule } from "../shared/sections/docs.ts";

export function getDocsPage() {
    return {
        slug: "docs",
        navigation: getPrimaryNavigation("docs"),
        section: getDocsModule(),
        insights: getSharedInsights("docs"),
    };
}
