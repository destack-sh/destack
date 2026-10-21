import { definePackage } from "@destack/object";
import { distribution } from "./object/index.ts";

/** The handle machines serve their distribution through, and clients call it by. */
export default definePackage({ objects: { distribution } });
