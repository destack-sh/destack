import { principal } from "@destack/access";
import { defineObject } from "./object.ts";

/** The clients people use Destack through, the scope each client's local objects live in. */
export const client = defineObject({
    name: "client",
    plural: "clients",
    scope: "universe",
    represents: principal.client,
    fields: {},
    permissions: [],
}).methods();
