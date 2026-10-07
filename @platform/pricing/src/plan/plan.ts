import { CREDITS, type DecimalAmount, type Sku } from "@destack/finance";
import type { ProductDefinition } from "@destack/finance/client";
import { defineSku } from "@destack/finance/declare";
import * as meters from "@destack/observability/meter";
import { CLOUDFLARE_COSTS, DURABLE_OBJECT_GIGABYTES } from "../vendor/cloudflare.ts";
import { capacityClasses, compute, customDomains, members, storage, support } from "./feature.ts";

/** The currencies Destack bills in: North America's and Europe's. */
export const CURRENCIES = ["USD", "CAD", "EUR", "CHF", "GBP"] as const;

/** A currency Destack bills in. */
export type BillingCurrency = (typeof CURRENCIES)[number];

/** Destack's price book: a credit's price in each currency it bills, in minor units of the currency per minor unit of credits. */
export const CREDIT_PRICES = {
    USD: "1",
    CAD: "1.3",
    EUR: "1",
    CHF: "1",
    GBP: "1",
} as const satisfies Readonly<Record<BillingCurrency, DecimalAmount>>;

/** A provider's list price for one of its pricing units, which a Destack unit's price is set from and its bill is reconciled against. */
export interface ProviderCost {
    /** The provider. */
    readonly provider: string;
    /** The provider's service, such as Durable Objects. */
    readonly service: string;
    /** The provider's pricing unit, such as GB-month. */
    readonly pricingUnit: string;
    /** The meter's units in one pricing unit, such as 10^9 byte-months in a GB-month, one for an operation. */
    readonly unitSize: number;
    /** The list price per pricing unit, in fractional US cents. */
    readonly unitAmount: string;
    /** The page publishing the price. */
    readonly source: string;
    /** The day the price was read at its source, as YYYY-MM-DD. */
    readonly observedAt: string;
}

/** The page publishing Destack's unit prices with their cost basis. */
const SOURCE = "https://destack.sh/pricing";

/** The day the unit prices were set. */
const OBSERVED_AT = "2026-10-07";

/** Database storage, 0.25 credits per GB-month, a GB taken as 10^9 bytes. */
export const databaseStorageUnit = defineSku({
    name: "destack.databaseStorage",
    description: "Database storage, kept on average over the month.",
    provider: "destack",
    service: "Destack Databases",
    category: "Databases",
    meter: meters.databaseStorage.reference,
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: CREDITS,
    unitAmount: "25",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Bucket storage, 0.02 credits per GB-month, a GB taken as 10^9 bytes. */
export const bucketStorageUnit = defineSku({
    name: "destack.bucketStorage",
    description: "Bucket storage, kept on average over the month.",
    provider: "destack",
    service: "Destack Buckets",
    category: "Storage",
    meter: meters.bucketStorage.reference,
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: CREDITS,
    unitAmount: "2",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Compute, 0.06 credits per GB-hour. */
export const computeUnit = defineSku({
    name: "destack.compute",
    description: "Compute, by memory held over time.",
    provider: "destack",
    service: "Destack Compute",
    category: "Compute",
    meter: meters.compute.reference,
    pricingUnit: "GB-hour",
    unitSize: 3600,
    currency: CREDITS,
    unitAmount: "6",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Operations, counted at their published prices in millionths of a credit, billed a credit per credit. */
export const operationUnit = defineSku({
    name: "destack.operations",
    description: "Requests and storage reads and writes, at each one's published price.",
    provider: "destack",
    service: "Destack Compute",
    category: "Compute",
    meter: meters.operations.reference,
    pricingUnit: "credit",
    unitSize: 1_000_000,
    currency: CREDITS,
    unitAmount: "100",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Egress, free, a GB taken as 10^9 bytes. */
export const egressUnit = defineSku({
    name: "destack.egress",
    description: "Bytes served out of Destack.",
    provider: "destack",
    service: "Destack Network",
    category: "Networking",
    meter: meters.egress.reference,
    pricingUnit: "GB",
    unitSize: 1_000_000_000,
    currency: CREDITS,
    unitAmount: "0",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** The units every cell rates its spaces' usage by, one per meter, the same price wherever the usage ran. */
export const UNIT_SKUS: readonly Sku[] = [
    databaseStorageUnit,
    bucketStorageUnit,
    computeUnit,
    operationUnit,
    egressUnit,
];

/** The price of each operation a cell counts, in millionths of a credit, each a quarter above its cost. */
export const OPERATION_PRICES = {
    /** A request to a Worker. */
    workerRequest: 0.375,
    /** A request to a Durable Object. */
    durableObjectRequest: 0.1875,
    /** A database row read. */
    rowRead: 0.00125,
    /** A database row written. */
    rowWrite: 1.25,
    /** A bucket write or list. */
    bucketWrite: 5.625,
    /** A bucket read. */
    bucketRead: 0.45,
} as const;

/** An operation a cell counts. */
export type OperationClass = keyof typeof OPERATION_PRICES;

/** Each unit with the provider cost its price is set from, which price pages show as its cost basis. */
export const UNIT_COSTS: readonly { readonly unit: Sku; readonly cost: ProviderCost }[] = [
    { unit: databaseStorageUnit, cost: CLOUDFLARE_COSTS.durableObjectStorage },
    { unit: bucketStorageUnit, cost: CLOUDFLARE_COSTS.r2Storage },
    { unit: computeUnit, cost: CLOUDFLARE_COSTS.durableObjectDuration },
    { unit: egressUnit, cost: CLOUDFLARE_COSTS.r2Egress },
];

/** Each operation with the provider cost its price is set from, which price pages show as its cost basis. */
export const OPERATION_COSTS: readonly {
    readonly operation: OperationClass;
    readonly cost: ProviderCost;
}[] = [
    { operation: "workerRequest", cost: CLOUDFLARE_COSTS.workerRequests },
    { operation: "durableObjectRequest", cost: CLOUDFLARE_COSTS.durableObjectRequests },
    { operation: "rowRead", cost: CLOUDFLARE_COSTS.durableObjectRowsRead },
    { operation: "rowWrite", cost: CLOUDFLARE_COSTS.durableObjectRowsWritten },
    { operation: "bucketWrite", cost: CLOUDFLARE_COSTS.r2ClassA },
    { operation: "bucketRead", cost: CLOUDFLARE_COSTS.r2ClassB },
];

/** Free: every account's default, 1 GB of storage and 100 hours of one Durable Object's compute a month, the usage paid by Destack. */
export const FREE: ProductDefinition = {
    name: "Free",
    description: "1 GB of storage and 100 hours of compute a month.",
    isDefault: true,
    prices: [],
    features: [
        storage.grant(1_000_000_000),
        compute.grant(100 * 60 * 60 * DURABLE_OBJECT_GIGABYTES),
        capacityClasses.grant(["elastic", "machine"]),
        support.grant("community"),
        members.grant(10),
    ],
};

/** Go: 8 a month of usage at Destack's unit prices, and more as it is used. */
export const GO: ProductDefinition = {
    name: "Go",
    description: "8 a month of usage at Destack's unit prices, and more as you use it.",
    prices: monthly("go", { USD: 800, CAD: 1100, EUR: 800, CHF: 800, GBP: 800 }),
    features: [
        storage.grant(null),
        compute.grant(null),
        capacityClasses.grant(["elastic", "machine"]),
        support.grant("community"),
        members.grant(10),
    ],
};

/** Plus: 20 a month of usage at Destack's unit prices, reserved capacity and custom domains, and more as it is used. */
export const PLUS: ProductDefinition = {
    name: "Plus",
    description: "20 a month of usage at Destack's unit prices, and more as you use it.",
    prices: monthly("plus", { USD: 2000, CAD: 2800, EUR: 2000, CHF: 2000, GBP: 2000 }),
    features: [
        storage.grant(null),
        compute.grant(null),
        capacityClasses.grant(["elastic", "reserved", "machine"]),
        support.grant("email"),
        customDomains.grant(null),
        members.grant(null),
    ],
};

/** Pro: 100 a month of usage at Destack's unit prices, reserved capacity and priority support, and more as it is used. */
export const PRO: ProductDefinition = {
    name: "Pro",
    description: "100 a month of usage at Destack's unit prices, and more as you use it.",
    prices: monthly("pro", { USD: 10_000, CAD: 14_000, EUR: 10_000, CHF: 10_000, GBP: 10_000 }),
    features: [
        storage.grant(null),
        compute.grant(null),
        capacityClasses.grant(["elastic", "reserved", "machine"]),
        support.grant("priority"),
        customDomains.grant(null),
        members.grant(null),
    ],
};

/** Destack's plans, cheapest first. */
export const PLANS: readonly ProductDefinition[] = [FREE, GO, PLUS, PRO];

/** Price a plan's monthly fee in each currency, each fee including as much usage, under lookup keys of the plan's key. */
function monthly(
    key: string,
    fees: Readonly<Record<BillingCurrency, number>>,
): ProductDefinition["prices"] {
    return CURRENCIES.map((currency) => ({
        lookupKey: `${key}-${currency.toLowerCase()}`,
        currency,
        unitAmount: fees[currency],
        includedUsage: fees[currency],
        recurring: { interval: "month" as const, intervalCount: 1, usage: "licensed" as const },
    }));
}
