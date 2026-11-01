import { definePackage } from "@destack/object";
import { cards } from "./board.ts";

/** The board: its cards, whose code runs in its sandbox. */
export default definePackage({ objects: { card: cards } });
