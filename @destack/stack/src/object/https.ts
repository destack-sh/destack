import { HttpsKind, defineObject } from "@destack/object";
import { space } from "@destack/account/object";

/** An external HTTPS API a workload calls, lent a connection's credential by the machine. */
export const https = defineObject({
    name: "https",
    plural: "httpsApis",
    scope: space,
    provisioned: { kind: HttpsKind },
    fields: {},
    administration: ["read"],
});
