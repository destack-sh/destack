import { getWidgetCountLabel } from "./foo.ts";
import { renderStatusCard } from "./ui/card.ts";

const widgetCountLabel = getWidgetCountLabel(42);
const statusCard = renderStatusCard("ready", widgetCountLabel);

console.info(`the answer is ${statusCard}`);
