export function renderDashboardCard(title, summary) {
    return `${title}:${summary}`;
}

export function formatDashboardHeading(activeTab) {
    return `dashboard:${activeTab}`;
}

export function renderDashboardSummary(userName, activeTab) {
    const dashboardHeading = formatDashboardHeading(activeTab);
    return `${dashboardHeading}:${userName}`;
}

export const sessionState = { userName: "florian", activeTab: "overview" };

const dashboardSummary = renderDashboardSummary(sessionState.userName, sessionState.activeTab);
const dashboardCard = renderDashboardCard("timeline", dashboardSummary);
console.log(dashboardCard);
//# sourceMappingURL=./app.js.map
