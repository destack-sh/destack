import { definePackage } from "@destack/package/declare";
import { inboxDatabase } from "./stack/index.ts";

/** The handle stacks import to install the inbox in a home. */
export default definePackage({ resources: { main: inboxDatabase } });
