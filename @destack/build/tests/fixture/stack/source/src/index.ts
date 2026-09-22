import { defineSpace } from "@destack/space";
import type {} from "@destack/package/import-meta";
import { defineAccount } from "@destack/model/declare";
import { defineObject, relation } from "@destack/access";
import { database } from "@destack/template-stack";

/** Reuse the database declaration from the shared stack package. */
export { database } from "@destack/template-stack";

/** Declare environments independently of the destination space. */
export const account = defineAccount({ environments: { development: {}, production: {} } });

/** Declare inspectable application permissions. */
export const note = defineObject({
    packageId: import.meta.destack.package.id,
    name: "note",
    attributes: {},
    relations: { owner: { kind: "subject", subjects: ["user"] } },
    permissions: { read: relation("owner") },
});

/** Configure a space without provisioning resources during compilation. */
export const personal = defineSpace({
    resources: {
        main: { declaration: database, retention: "retain", tags: {} },
    },
});
