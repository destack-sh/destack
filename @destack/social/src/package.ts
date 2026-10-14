import { definePackage } from "@destack/runtime";
import { comment, favourite, presence, reaction, receipt } from "./object/index.ts";

/** The social package: comments, reactions, read receipts, favourites and presence. */
export default definePackage({ objects: { comment, reaction, receipt, favourite, presence } });
