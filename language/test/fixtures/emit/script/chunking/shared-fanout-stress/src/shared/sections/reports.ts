import { getCommonCards } from "./shared-cards.ts";

export function getReportsModule() {
    return {
        heading: "reports",
        cards: [...getCommonCards("reports"), "reports-cohort"],
    };
}
