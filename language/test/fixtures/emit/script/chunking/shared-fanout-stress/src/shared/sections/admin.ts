import { getCommonCards } from "./shared-cards.ts";

export function getAdminModule() {
    return {
        heading: "admin",
        cards: [...getCommonCards("admin"), "admin-members"],
    };
}
