import { definePackage } from "@destack/runtime";
import { page } from "./object/index.ts";

/** The handle stacks import to install pages in a space, and clients call its pages through. */
export default definePackage({ objects: { page } });
