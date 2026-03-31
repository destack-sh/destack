import { renderDashboardSummary } from "./pages/dashboard/render.ts";
import { renderDashboardCard } from "./pages/dashboard/card.ts";
import { sessionState } from "./state/session.ts";

const dashboardSummary = renderDashboardSummary(
    sessionState.userName,
    sessionState.activeTab,
);
const dashboardCard = renderDashboardCard("timeline", dashboardSummary);

console.log(dashboardCard);
