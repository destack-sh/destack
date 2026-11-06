import { definePackage } from "@destack/object";
import { cards, ideas, retitle } from "./board.ts";

/** The board: its cards, whose code runs in its sandbox, and the commands on them. */
export default definePackage({
    objects: { card: cards },
    commands: { "retitle-ideas": retitle, "new-ideas": ideas },
});
