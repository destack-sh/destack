export function renderStatusCard(status, widgetCountLabel) {
    return `${status}:${widgetCountLabel}`;
}

export function formatWidgetCount(widgetCount) {
    return `widgets:${widgetCount}`;
}

export function getWidgetCountLabel(widgetCount) {
    return formatWidgetCount(widgetCount);
}

const widgetCountLabel = getWidgetCountLabel(42);
const statusCard = renderStatusCard("ready", widgetCountLabel);
console.info(`the answer is ${statusCard}`);
//# sourceMappingURL=./bundle.js.map
