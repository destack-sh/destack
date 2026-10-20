import { t } from "@destack/locale";
import type { StackDefinition } from "@destack/stack/declare";
import { Fee } from "../price/book.ts";
import {
    compute,
    customDomains,
    database,
    earlyAccess,
    network,
    retention,
    storage,
    support,
    tenancy,
} from "./feature.ts";

/** The bytes the free plan keeps in databases: 1 GB, a GB taken as 10^9 bytes. */
const FREE_DATABASE = 1_000_000_000;

/** The bytes the free plan keeps in buckets: 10 GB, a GB taken as 10^9 bytes. */
const FREE_STORAGE = 10_000_000_000;

/** The GB-seconds the free plan runs each month: 128 MB held 3.25 hours a day, 1.125 credits. */
const FREE_COMPUTE = 45_000;

/** The bytes the free plan sends out each month: 100 GB, its 10 GB of storage served ten times. */
const FREE_NETWORK = 100_000_000_000;

/** The days each plan keeps events, after Railway's 7, 30 and 90 day log history, then a year. */
const RETENTION_DAYS = { free: 7, go: 30, plus: 90, pro: 365 } as const;

/** Destack's plans by key, cheapest first, each fee a round figure set in each currency. */
export const PLANS = {
    // free: every account's default, a small allowance on shared machines, capped
    free: {
        name: t`Free`,
        description: t`All of Destack, within a free allowance.`,
        isDefault: true,
        productFeatures: {
            database: database.grant(FREE_DATABASE),
            storage: storage.grant(FREE_STORAGE),
            compute: compute.grant(FREE_COMPUTE),
            network: network.grant(FREE_NETWORK),
            tenancy: tenancy.grant(["shared", "own"]),
            support: support.grant("community"),
            retention: retention.grant(RETENTION_DAYS.free),
        },
    },
    // go: its fee in usage, then pay as you go, with early access
    go: {
        name: t`Go`,
        description: t`All of Destack without caps, with early access.`,
        prices: Fee.prices({ USD: 800, CAD: 1100, EUR: 800, CHF: 800, GBP: 800, AUD: 1200 }),
        productFeatures: {
            database: database.grant(null),
            storage: storage.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "own"]),
            support: support.grant("community"),
            retention: retention.grant(RETENTION_DAYS.go),
            "early-access": earlyAccess.grant(null),
        },
    },
    // plus: more usage in its fee, with custom domains and email support
    plus: {
        name: t`Plus`,
        description: t`Go, with custom domains and email support.`,
        prices: Fee.prices({ USD: 2000, CAD: 2800, EUR: 2000, CHF: 2000, GBP: 2000, AUD: 3000 }),
        productFeatures: {
            database: database.grant(null),
            storage: storage.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "own"]),
            support: support.grant("email"),
            "custom-domains": customDomains.grant(null),
            retention: retention.grant(RETENTION_DAYS.plus),
            "early-access": earlyAccess.grant(null),
        },
    },
    // pro: the most usage in its fee, with dedicated machines and priority support
    pro: {
        name: t`Pro`,
        description: t`Plus, with dedicated machines and priority support.`,
        prices: Fee.prices({
            USD: 10_000,
            CAD: 14_000,
            EUR: 10_000,
            CHF: 10_000,
            GBP: 10_000,
            AUD: 15_000,
        }),
        productFeatures: {
            database: database.grant(null),
            storage: storage.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "dedicated", "own"]),
            support: support.grant("priority"),
            "custom-domains": customDomains.grant(null),
            retention: retention.grant(RETENTION_DAYS.pro),
            "early-access": earlyAccess.grant(null),
        },
    },
} satisfies StackDefinition["products"];
