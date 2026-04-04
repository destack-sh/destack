import { getSharedSectionList } from "./sections.ts";

export function getMarketingFeatureSet() {
    return [...getSharedSectionList(), "signup-cta", "roi-proof"];
}
