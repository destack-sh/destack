import { defineSpace } from "@destack/space";
import { database } from "@destack/template-stack";

/** Reuse the database declaration from the shared stack package. */
export { database } from "@destack/template-stack";

/** Configure a space without provisioning resources during compilation. */
export const personal = defineSpace({
    resources: {
        main: { declaration: database, retention: "retain", tags: {} },
    },
});
