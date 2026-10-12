import { t } from "@destack/locale";
import type { StackDefinition } from "@destack/stack/declare";
import { Fee } from "../price/book.ts";
import { compute, customDomains, database, network, storage, support, tenancy } from "./feature.ts";

/** The bytes the free plan keeps in databases: 1 GB, a GB taken as 10^9 bytes. */
const FREE_DATABASE = 1_000_000_000;

/** The bytes the free plan keeps in buckets: 10 GB, a GB taken as 10^9 bytes. */
const FREE_STORAGE = 10_000_000_000;

/** The GB-seconds the free plan runs each month: 128 MB held 3.25 hours a day, 0.75 credits. */
const FREE_COMPUTE = 45_000;

/** The bytes the free plan sends out each month: 100 GB, its 10 GB of storage served ten times. */
const FREE_NETWORK = 100_000_000_000;

/** Destack's plans by key, cheapest first, each fee a round figure set in each currency. */
export const PLANS = {
    // TODO #Incomplete: the final copy replaces these placeholder names and descriptions

    // free: every account's default, a small allowance on shared machines, capped
    free: {
        name: t`Free`,
        description: t`Meet Destack.`,
        isDefault: true,
        productFeatures: {
            database: database.grant(FREE_DATABASE),
            storage: storage.grant(FREE_STORAGE),
            compute: compute.grant(FREE_COMPUTE),
            network: network.grant(FREE_NETWORK),
            tenancy: tenancy.grant(["shared", "own"]),
            support: support.grant("community"),
        },
    },
    // go: its fee in usage, then pay as you go, under the account's own domains
    go: {
        name: t`Go`,
        description: t`More room to build.`,
        prices: Fee.prices({ USD: 800, CAD: 1100, EUR: 800, CHF: 800, GBP: 800 }),
        productFeatures: {
            database: database.grant(null),
            storage: storage.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "own"]),
            support: support.grant("community"),
            "custom-domains": customDomains.grant(null),
        },
    },
    // plus: its fee in usage, then pay as you go, unlocking dedicated machines
    plus: {
        name: t`Plus`,
        description: t`More power, on machines of your own.`,
        prices: Fee.prices({ USD: 2000, CAD: 2800, EUR: 2000, CHF: 2000, GBP: 2000 }),
        productFeatures: {
            database: database.grant(null),
            storage: storage.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "dedicated", "own"]),
            support: support.grant("email"),
            "custom-domains": customDomains.grant(null),
        },
    },
    // pro: its fee in usage, then pay as you go, unlocking dedicated machines, with priority support
    pro: {
        name: t`Pro`,
        description: t`The most out of Destack.`,
        prices: Fee.prices({ USD: 10_000, CAD: 14_000, EUR: 10_000, CHF: 10_000, GBP: 10_000 }),
        productFeatures: {
            database: database.grant(null),
            storage: storage.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "dedicated", "own"]),
            support: support.grant("priority"),
            "custom-domains": customDomains.grant(null),
        },
    },
} satisfies StackDefinition["products"];
