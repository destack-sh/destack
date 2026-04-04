import { formatWidgetCount } from "./helpers/count.ts";

/** Build one widget count label. */
export function getWidgetCountLabel(widgetCount: number) {
    return formatWidgetCount(widgetCount);
}
