export function renderStatusSummary(title, card) {
    return `${title}:${card}`;
}

export function renderStatusCard(title, badge) {
    return `${title}:${badge}`;
}

export function renderStatusBadge(status) {
    return `[${status.toUpperCase()}]`;
}

const statusBadge = renderStatusBadge("ready");
const statusCard = renderStatusCard("emit", statusBadge);
const statusSummary = renderStatusSummary("emit", statusCard);
console.log(statusSummary);
//# sourceMappingURL=./bundle.js.map
