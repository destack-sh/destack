import { Tenancy } from "@destack/account/object";
import { defineFeature } from "@destack/finance/declare";
import * as meters from "@destack/observability/meter";
import { schema } from "@destack/schema";

/** The bytes an account keeps in its databases, capped at the bytes kept now. */
export const database = defineFeature({
    name: "database",
    description: "Bytes kept in databases.",
    kind: "metered",
    resource: "database",
    meters: [meters.databaseBytes.reference],
    reset: "never",
});

/** The bytes an account keeps in its buckets, capped at the bytes kept now. */
export const storage = defineFeature({
    name: "storage",
    description: "Bytes kept in buckets.",
    kind: "metered",
    resource: "storage",
    meters: [meters.storageBytes.reference],
    reset: "never",
});

/** The GB-seconds an account's installations run per billing period. */
export const compute = defineFeature({
    name: "compute",
    description: "GB-seconds run per billing period.",
    kind: "metered",
    resource: "compute",
    meters: [meters.memory.reference],
    reset: "period",
});

/** The bytes an account's spaces send out per billing period. */
export const network = defineFeature({
    name: "network",
    description: "Bytes sent out per billing period.",
    kind: "metered",
    resource: "network",
    meters: [meters.egress.reference],
    reset: "period",
});

/** Where an account's spaces may run. */
export const tenancy = defineFeature({
    name: "tenancy",
    description: "Where the account's spaces may run.",
    kind: "static",
    resource: "tenancy",
    value: schema.array(Tenancy),
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
    resource: "domain",
});

/** The days an account's events stay, such as activity, history and logs, before they expire. */
export const retention = defineFeature({
    name: "retention",
    description: "Days events stay before they expire.",
    kind: "static",
    resource: "retention",
    value: schema.number().int().positive(),
});

/** New apps and features before everyone else. */
export const earlyAccess = defineFeature({
    name: "earlyAccess",
    description: "New apps and features before everyone else.",
    kind: "boolean",
});
