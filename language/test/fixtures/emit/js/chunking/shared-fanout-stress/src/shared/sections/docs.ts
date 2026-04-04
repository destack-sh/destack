import { getCommonCards } from "./shared-cards.ts";

export function getDocsModule() {
    return {
        heading: "docs",
        cards: [...getCommonCards("docs"), "docs-guides"],
    };
}
