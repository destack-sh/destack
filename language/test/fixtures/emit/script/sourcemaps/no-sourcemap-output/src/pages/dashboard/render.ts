import { formatDashboardHeading } from "./strings.ts";

/** Render one dashboard summary string. */
export function renderDashboardSummary(userName: string, activeTab: string) {
    const dashboardHeading = formatDashboardHeading(activeTab);

    return `${dashboardHeading}:${userName}`;
}
