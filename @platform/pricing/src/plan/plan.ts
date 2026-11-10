import { t } from "@destack/locale";
import type { CatalogProduct } from "@destack/finance/declare";
import { Fee } from "../price/book.ts";
import {
    bucket,
    compute,
    customDomains,
    customRetention,
    database,
    earlyAccess,
    network,
    support,
    tenancy,
} from "./feature.ts";

/** The credit cents of usage the free plan includes per account each month. */
const FREE_USAGE =
    // add up 1 GB of database, 10 GB of files, 10 GB-hours of compute and a million requests
    40 + 30 + 90 + 90;

/** Destack's plans by key, cheapest first, each fee per member a round figure in each currency. */
export const PLANS = {
    // free: every account's default, its usage per account included on shared machines, then blocked
    free: {
        name: t`Free`,
        description: t`For your first apps, with all of Destack within its included usage.`,
        isDefault: true,
        includedUsage: FREE_USAGE,
        overage: "capped",
        productFeatures: {
            database: database.grant(null),
            bucket: bucket.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "local"]),
            support: support.grant("community"),
        },
    },
    // go: its fee in usage, then capped, with custom retention and early access
    go: {
        name: t`Go`,
        description: t`For your everyday stack, with all of Destack, custom retention and early access.`,
        overage: "capped",
        prices: Fee.prices({ USD: 800, CAD: 1100, EUR: 800, CHF: 800, GBP: 800, AUD: 1300 }),
        productFeatures: {
            database: database.grant(null),
            bucket: bucket.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "local"]),
            support: support.grant("community"),
            "custom-retention": customRetention.grant(null),
            "early-access": earlyAccess.grant(null),
        },
    },
    // plus: more usage in its fee, then pay as you go, with custom domains and email support
    plus: {
        name: t`Plus`,
        description: t`For your daily work, with custom domains and email support.`,
        prices: Fee.prices({ USD: 2000, CAD: 2800, EUR: 2000, CHF: 2000, GBP: 2000, AUD: 3200 }),
        productFeatures: {
            database: database.grant(null),
            bucket: bucket.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "local"]),
            support: support.grant("email"),
            "custom-domains": customDomains.grant(null),
            "custom-retention": customRetention.grant(null),
            "early-access": earlyAccess.grant(null),
        },
    },
    // pro: the most usage in its fee, with dedicated machines and priority support
    pro: {
        name: t`Pro`,
        description: t`For your critical software, with dedicated machines and priority support.`,
        prices: Fee.prices({ USD: 5000, CAD: 7000, EUR: 5000, CHF: 5000, GBP: 5000, AUD: 8000 }),
        productFeatures: {
            database: database.grant(null),
            bucket: bucket.grant(null),
            compute: compute.grant(null),
            network: network.grant(null),
            tenancy: tenancy.grant(["shared", "dedicated", "local"]),
            support: support.grant("priority"),
            "custom-domains": customDomains.grant(null),
            "custom-retention": customRetention.grant(null),
            "early-access": earlyAccess.grant(null),
        },
    },
} satisfies Readonly<Record<string, CatalogProduct>>;

/** The contract organisations sign past Pro, its fee a monthly commitment spent on usage. */
export const ENTERPRISE = {
    name: t`Enterprise`,
    description: t`For your organisation, with SLAs, onboarding, invoice billing and a data processing agreement.`,
    support: "sla",
    commitment: {
        USD: 100_000,
        CAD: 140_000,
        EUR: 100_000,
        CHF: 100_000,
        GBP: 100_000,
        AUD: 160_000,
    } satisfies Fee,
} as const;
