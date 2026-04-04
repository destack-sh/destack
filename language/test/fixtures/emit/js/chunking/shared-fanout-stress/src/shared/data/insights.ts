import { getInsightBadge } from "./labels.ts";

export function getSharedInsights(slug: string) {
    return [
        `${slug}:${getInsightBadge("alpha")}`,
        `${slug}:${getInsightBadge("beta")}`,
        `${slug}:${getInsightBadge("gamma")}`,
    ];
}
