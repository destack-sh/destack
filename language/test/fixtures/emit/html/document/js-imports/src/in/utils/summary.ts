import { greeting } from "./strings.ts";
import { padZero } from "./numbers.ts";

export function renderSummary(name: string) {
    return `${greeting(name)}:${padZero(7)}`;
}
