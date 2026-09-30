import { definePackage } from "@destack/package/declare";
import { pagesDatabase } from "./stack/index.ts";

/** The handle stacks import to install this package. */
export default definePackage({ resources: { main: pagesDatabase } });
