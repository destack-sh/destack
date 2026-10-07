import { CapacityClass } from "@destack/account/object";
import { defineFeature } from "@destack/finance/declare";
import * as meters from "@destack/observability/meter";
import { schema } from "@destack/schema";

/** The bytes an account keeps in its databases and buckets together, capped at the bytes kept now. */
export const storage = defineFeature({
    name: "storage",
    description: "Bytes kept in databases and buckets.",
    kind: "metered",
    allowance: "storage",
    meters: [meters.databaseStorage.reference, meters.bucketStorage.reference],
    reset: "never",
});

/** The GB-seconds an account's installations run per billing period. */
export const compute = defineFeature({
    name: "compute",
    description: "GB-seconds run per billing period.",
    kind: "metered",
    allowance: "compute",
    meters: [meters.compute.reference],
    reset: "period",
});

/** The capacity classes an account's spaces may run in. */
export const capacityClasses = defineFeature({
    name: "capacityClasses",
    description: "Capacity classes the account's spaces may run in.",
    kind: "static",
    allowance: "capacity",
    value: schema.array(CapacityClass),
});

/** The help people give an account. */
export const support = defineFeature({
    name: "support",
    description: "The help people give the account.",
    kind: "static",
    value: schema.enum(["community", "email", "priority"]),
});

/** Serving an account's spaces under its own domains. */
export const customDomains = defineFeature({
    name: "customDomains",
    description: "Serving spaces under the account's own domains.",
    kind: "boolean",
    allowance: "domain",
});

/** The most members an organisation account holds, unlimited when null; members are never billed per seat. */
export const members = defineFeature({
    name: "members",
    description: "The most members an organisation holds; members are never billed per seat.",
    kind: "static",
    allowance: "membership",
    value: schema.int().positive().nullable(),
});
