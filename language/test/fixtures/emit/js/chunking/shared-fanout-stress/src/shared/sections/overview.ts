import { getCommonCards } from "./shared-cards.ts";

export function getOverviewModule() {
    return {
        heading: "overview",
        cards: [...getCommonCards("overview"), "overview-usage"],
    };
}
