import { defineObject } from "@destack/object";

/** The spaces settings recommend values in, owned through their owner role. */
export const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: "universe",
    isScope: true,
    fields: {},
    permissions: ["read", "write"],
});
