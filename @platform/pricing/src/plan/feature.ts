import { Tenancy } from "@destack/account/object";
import { defineFeature } from "@destack/finance/declare";
import * as meters from "@destack/telemetry/meter";
import { schema } from "@destack/schema";
import { t } from "@destack/locale";

/** The bytes an account keeps in its databases, capped at the bytes kept now. */
export const database = defineFeature({
    name: "database",
    description: t`Bytes kept in databases.`,
    kind: "metered",
    resource: "database",
    meters: [meters.databaseUsage],
    reset: "never",
});

/** The bytes an account keeps in its buckets, capped at the bytes kept now. */
export const bucket = defineFeature({
    name: "bucket",
    description: t`Bytes kept in buckets.`,
    kind: "metered",
    resource: "bucket",
    meters: [meters.bucketUsage],
    reset: "never",
});

/** The GB-seconds an account's installations run per billing period. */
export const compute = defineFeature({
    name: "compute",
    description: t`GB-seconds run per billing period.`,
    kind: "metered",
    resource: "compute",
    meters: [meters.computeUsage],
    reset: "period",
});

/** The bytes an account's spaces move in and out per billing period. */
export const network = defineFeature({
    name: "network",
    description: t`Bytes moved in and out per billing period.`,
    kind: "metered",
    resource: "network",
    meters: [meters.networkTransfer],
    reset: "period",
});

/** Where an account's spaces may run. */
export const tenancy = defineFeature({
    name: "tenancy",
    description: t`Where the account's spaces may run.`,
    kind: "static",
    resource: "tenancy",
    value: schema.array(Tenancy),
});

/** The help people give an account. */
export const support = defineFeature({
    name: "support",
    description: t`The help people give the account.`,
    kind: "static",
    value: schema.enum(["email", "priority"]),
});

/** Serving an account's spaces under its own domains. */
export const customDomains = defineFeature({
    name: "customDomains",
    description: t`Serving spaces under the account's own domains.`,
    kind: "boolean",
    resource: "domain",
});

/** Choosing how long an account's spaces keep events, past each setting's default. */
export const customRetention = defineFeature({
    name: "customRetention",
    description: t`Choosing how long spaces keep events, past each setting's default.`,
    kind: "boolean",
    resource: "retention",
});

/** New apps and features before everyone else. */
export const earlyAccess = defineFeature({
    name: "earlyAccess",
    description: t`New apps and features before everyone else.`,
    kind: "boolean",
});
