import { definePackage } from "@destack/package/declare";
import { database } from "@destack/template-stack";

/** The handle stacks import to install this package. */
export default definePackage({ resources: { main: database } });
