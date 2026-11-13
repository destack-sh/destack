import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { cards, ideas, retitle } from "./board.ts";

/** The board: its cards, whose code runs in its sandbox, and the commands on them. */
export default definePackage({
    title: t`Board`,
    description: t`Pin cards to a board.`,
    icon: "kanban",
    color: "iris",
    objects: [cards],
    commands: [retitle, ideas],
});
