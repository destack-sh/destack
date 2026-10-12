import { Sku } from "@destack/finance";
import * as meters from "@destack/observability/meter";
import { PricingModel } from "../price/model.ts";

/** The package keeping the vendors' list prices as data. */
const OWNER = import.meta.destack.package;

/** The page publishing Cloud Storage's prices. */
const STORAGE = "https://cloud.google.com/storage/pricing";

/** The page publishing Firestore's prices. */
const FIRESTORE = "https://cloud.google.com/firestore/pricing";

/** The page publishing Cloud Run's prices. */
const RUN = "https://cloud.google.com/run/pricing";

/** The page publishing network prices. */
const NETWORK = "https://cloud.google.com/vpc/network-pricing";

/** The day the prices were read at their sources. */
const OBSERVED_AT = "2026-10-12";

/** The region the prices apply in: Iowa. */
const REGION = "us-central1";

/** The bytes in a GiB, the gibibyte Google prices storage and transfer in. */
const GIBIBYTE = 1_073_741_824;

/** The hours in Google's billing month, which its monthly prices multiply hourly ones by. */
const HOURS_PER_MONTH = 730;

/** Cloud Storage Standard regional storage, $0.000027397 per GiB-hour. */
export const storage = new Sku(OWNER, {
    name: "gcp.storage",
    description: "Cloud Storage Standard storage in a US region.",
    provider: "gcp",
    region: REGION,
    service: "Cloud Storage",
    category: "Storage",
    meter: meters.storageBytes.reference,
    pricingUnit: "GiB-hour",
    unitSize: GIBIBYTE / HOURS_PER_MONTH,
    currency: "USD",
    unitAmount: "0.0027397",
    source: STORAGE,
    observedAt: OBSERVED_AT,
});

/** Cloud Storage Standard regional Class A operations, $0.005 per 1,000. */
export const storageWrites = new Sku(OWNER, {
    name: "gcp.storageWrites",
    description: "Cloud Storage Standard regional Class A operations: writes and lists.",
    provider: "gcp",
    region: REGION,
    service: "Cloud Storage",
    category: "Storage",
    meter: meters.storageWrites.reference,
    pricingUnit: "operation",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0005",
    source: STORAGE,
    observedAt: OBSERVED_AT,
});

/** Cloud Storage Standard regional Class B operations, $0.0004 per 1,000. */
export const storageReads = new Sku(OWNER, {
    name: "gcp.storageReads",
    description: "Cloud Storage Standard regional Class B operations: reads.",
    provider: "gcp",
    region: REGION,
    service: "Cloud Storage",
    category: "Storage",
    meter: meters.storageReads.reference,
    pricingUnit: "operation",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.00004",
    source: STORAGE,
    observedAt: OBSERVED_AT,
});

/** Firestore Standard edition stored data, $0.000205479 per GiB-hour. */
export const firestoreStorage = new Sku(OWNER, {
    name: "gcp.firestoreStorage",
    description: "Firestore Standard edition stored data.",
    provider: "gcp",
    region: REGION,
    service: "Cloud Firestore",
    category: "Databases",
    meter: meters.databaseBytes.reference,
    pricingUnit: "GiB-hour",
    unitSize: GIBIBYTE / HOURS_PER_MONTH,
    currency: "USD",
    unitAmount: "0.0205479",
    source: FIRESTORE,
    observedAt: OBSERVED_AT,
});

/** Cloud Run requests under request-based billing, $0.40 per million. */
export const runRequests = new Sku(OWNER, {
    name: "gcp.runRequests",
    description: "Cloud Run services requests under request-based billing.",
    provider: "gcp",
    region: REGION,
    service: "Cloud Run",
    category: "Compute",
    meter: meters.requests.reference,
    pricingUnit: "request",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.00004",
    source: RUN,
    observedAt: OBSERVED_AT,
});

/** Premium Tier data transfer out to the internet within the Americas, $0.12 per GiB. */
export const egress = new Sku(OWNER, {
    name: "gcp.egress",
    description: "Premium Tier internet data transfer out from the Americas to the Americas.",
    provider: "gcp",
    region: REGION,
    service: "Networking",
    category: "Networking",
    meter: meters.egress.reference,
    pricingUnit: "GiB",
    unitSize: GIBIBYTE,
    currency: "USD",
    unitAmount: "12",
    source: NETWORK,
    observedAt: OBSERVED_AT,
});

/** Google Cloud's SKUs Destack's prices compare with. */
export const GCP_SKUS: readonly Sku[] = [
    storage,
    storageWrites,
    storageReads,
    firestoreStorage,
    runRequests,
    egress,
];

/** Google Cloud's price per unit of each meter, at Cloud Run, Firestore and Cloud Storage. */
export const GCP_PRICING = new PricingModel([
    // compute
    [meters.requests.reference, [[runRequests, 1]]],

    // database
    [meters.databaseBytes.reference, [[firestoreStorage, 1]]],

    // storage
    [meters.storageBytes.reference, [[storage, 1]]],
    [meters.storageWrites.reference, [[storageWrites, 1]]],
    [meters.storageReads.reference, [[storageReads, 1]]],

    // network
    [meters.egress.reference, [[egress, 1]]],
]);
