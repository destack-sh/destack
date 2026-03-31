import { getSharedSummary } from "./summary.ts";

export function renderPage(name: string) {
    return `${name}:${getSharedSummary()}`;
}
