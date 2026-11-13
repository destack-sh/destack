import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { cards } from "./board.ts";

/** The board: its cards, whose code runs in its sandbox. */
export default definePackage({
    title: t`Mentions`,
    description: t`Mention people on cards.`,
    icon: "at",
    color: "pink",
    objects: [cards],
});
