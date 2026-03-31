import { renderStatusBadge } from "./ui/render-badge.ts";
import { renderStatusCard } from "./ui/render-card.ts";
import { renderStatusSummary } from "./ui/render-summary.ts";

const statusBadge = renderStatusBadge("ready");
const statusCard = renderStatusCard("emit", statusBadge);
const statusSummary = renderStatusSummary("emit", statusCard);

console.log(statusSummary);
