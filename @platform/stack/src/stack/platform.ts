import { install } from "@destack/stack/declare";
import { defineCatalog, stripe } from "@destack/finance/declare";
import { REGION_MERCHANT } from "@destack/account/object";
import connectionPackage from "@destack/connection/package";
import financePackage from "@destack/finance/package";
import { defineStack } from "@destack/stack";
import { HOME_OPTIONS } from "./home.ts";
import sitePackage from "@platform/site/package";
import { defineVault } from "@destack/vault/declare";
import { CREDIT_PRICES, PLANS } from "@platform/pricing";

/** The platform's vault in its home, whose secrets the operator writes once. */
export const platformVault = defineVault({ name: "platform" });

/** The stack of the platform's home: a home with finance, Stripe and the site. */
export const PLATFORM_STACK = defineStack({
    ...HOME_OPTIONS,
    types: [...HOME_OPTIONS.types, connectionPackage, financePackage],
    sellers: { platform: { provider: "stripe", creditPrices: CREDIT_PRICES } },
    ...defineCatalog(PLANS),
    vaults: { ...HOME_OPTIONS.vaults, platform: platformVault.provision() },
    // the platform's Stripe account, whose key the deploy rotates in
    connections: {
        stripe: {
            ...stripe.provision(),
            protocol: { kind: "key", header: "authorization", scheme: "Bearer" },
        },
    },
    secrets: { "stripe-webhook": { parent: "platform", name: "stripe-webhook" } },
    installations: {
        ...HOME_OPTIONS.installations,
        finance: install(
            financePackage,
            { stripe: "stripe", "stripe-webhook": "stripe-webhook" },
            { isolation: "host", name: REGION_MERCHANT },
        ),
        site: install(sitePackage, {}, { name: "site", version: { kind: "host" } }),
    },
});
