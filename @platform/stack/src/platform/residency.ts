import financePackage, { FINANCE_INSTALLATION } from "@destack/finance/package";
import forgePackage, { FORGE_INSTALLATION } from "@destack/forge/package";
import { credentialVault, packageBucket, repositoryStorage } from "@destack/forge/stack";
import observabilityPackage from "@destack/observability/package";
import relayPackage from "@destack/relay/package";
import { defineStack, install } from "@destack/stack";
import { defineVault } from "@destack/vault/declare";
import { CREDIT_PRICES, PLANS } from "@platform/pricing";

/** The platform account's vault in a residency's home, whose secrets the operator writes once. */
export const platformVault = defineVault({ name: "platform", spec: {} });

/** The stack of each residency home: the public registry everyone finds, and the platform's billing. */
export const RESIDENCY_STACK = defineStack({
    sellers: { platform: { provider: "stripe", creditPrices: CREDIT_PRICES } },
    products: PLANS,
    resources: {
        platform: { declaration: platformVault, retention: "forever", tags: {} },
        packages: { declaration: packageBucket, retention: "forever", tags: {} },
        repositories: { declaration: repositoryStorage, retention: "forever", tags: {} },
        credentials: { declaration: credentialVault, retention: "forever", tags: {} },
    },
    secrets: {
        "github-webhook": { vault: "platform", name: "github-webhook" },
        "github-private-key": { vault: "platform", name: "github-private-key" },
        "stripe-secret-key": { vault: "platform", name: "stripe-secret-key" },
    },
    installations: {
        forge: install(
            forgePackage,
            {
                packages: "packages",
                repositories: "repositories",
                credentials: "credentials",
                "github-webhook": "github-webhook",
                "github-private-key": "github-private-key",
            },
            { name: FORGE_INSTALLATION },
        ),
        finance: install(
            financePackage,
            { "stripe-secret-key": "stripe-secret-key" },
            { name: FINANCE_INSTALLATION },
        ),
        relay: install(relayPackage, {}, { name: "relay" }),
        observability: install(observabilityPackage, {}, { name: "observability" }),
    },
    relationships: { everyone: { relation: "guest", subject: { anyone: true } } },
});
