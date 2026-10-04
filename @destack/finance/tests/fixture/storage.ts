import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { defineFeature, defineMeter } from "../../src/declare/index.ts";

/** The release of Carol's shop package declaring the fixture's features and meters. */
export const storage = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000101",
    name: "@shop/storage",
    version: "2026.9.0",
});

/** The API requests a storage account makes. */
export const requests = defineMeter(
    {
        name: "api.requests",
        description: "Requests to the storage API.",
        aggregation: "sum",
        unit: "request",
    },
    { package: storage },
);

/** The bytes a storage account keeps, as last measured. */
export const stored = defineMeter(
    {
        name: "storage.bytes",
        description: "Bytes kept in storage.",
        aggregation: "last",
        unit: "byte",
    },
    { package: storage },
);

/** Syncing across devices. */
export const sync = defineFeature(
    { name: "sync", description: "Sync files across devices.", kind: "boolean" },
    { package: storage },
);

/** The plan's support tier. */
export const support = defineFeature(
    {
        name: "support",
        description: "The support tier.",
        kind: "static",
        value: schema.enum(["community", "priority"]),
    },
    { package: storage },
);

/** The seats an account may fill. */
export const seats = defineFeature(
    {
        name: "seats",
        description: "The seats an account may fill.",
        kind: "static",
        value: schema.number().int().min(1),
    },
    { package: storage },
);

/** API requests per billing period. */
export const calls = defineFeature(
    {
        name: "api.calls",
        description: "API requests per billing period.",
        kind: "metered",
        meter: requests.reference,
        reset: "period",
    },
    { package: storage },
);

/** Bytes kept, never reset. */
export const quota = defineFeature(
    {
        name: "storage.quota",
        description: "Bytes kept in storage.",
        kind: "metered",
        meter: stored.reference,
        reset: "never",
    },
    { package: storage },
);

/** The release of a platform package, published by the destack account. */
export const hosting = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000102",
    name: "@destack/hosting",
    version: "2026.9.0",
});

/** Custom domains on hosted apps. */
export const domains = defineFeature(
    { name: "domains", description: "Serve apps on custom domains.", kind: "boolean" },
    { package: hosting },
);
