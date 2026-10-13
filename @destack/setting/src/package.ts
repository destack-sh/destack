import { definePackage } from "@destack/runtime";
import { setting } from "./object/index.ts";

/** The handle stores serve setting values through, and clients call them by. */
export default definePackage({ objects: { setting } });
