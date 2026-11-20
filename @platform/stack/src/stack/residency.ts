import { install } from "@destack/stack/declare";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { connectionVault } from "@destack/connection/stack";
import { defineCatalog, stripe } from "@destack/finance/declare";
import financePackage, { FINANCE_INSTALLATION } from "@destack/finance/package";
import forgePackage, { FORGE_INSTALLATION } from "@destack/forge/package";
import { credentialVault, repositoryStorage } from "@destack/forge/stack";
import { gitStorage } from "@destack/stack/object";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import relayPackage from "@destack/relay/package";
import socialPackage from "@destack/social/package";
import { defineStack, type StackInput } from "@destack/stack";
import blankPackage from "@destack/blank/package";
import sitePackage from "@platform/site/package";
import { HOME_APPLICATION } from "@destack/account/object";
import { defineVault } from "@destack/vault/declare";
import vaultPackage from "@destack/vault/package";
import { CREDIT_PRICES, PLANS } from "@platform/pricing";

/** The platform account's vault in a residency's home, whose secrets the operator writes once. */
export const platformVault = defineVault({ name: "platform" });

/** Declare a residency home's stack: the registry, billing, its relay and its own apps. */
function residencyStack(own: NonNullable<StackInput["installations"]>) {
    return defineStack({
        types: [financePackage, vaultPackage, connectionPackage, forgePackage, gitStorage],
        sellers: { platform: { provider: "stripe", creditPrices: CREDIT_PRICES } },
        ...defineCatalog(PLANS),
        vaults: {
            platform: platformVault.provision(),
            credentials: credentialVault.provision(),
            connections: connectionVault.provision(),
        },
        // the platform's Stripe account, whose key the deploy rotates in
        connections: {
            stripe: {
                ...stripe.provision(),
                protocol: { kind: "key", header: "authorization", scheme: "Bearer" },
            },
        },
        gitStorages: { repositories: repositoryStorage.provision() },
        issuers: { github: {} },
        secrets: {
            "github-webhook": { parent: "platform", name: "github-webhook" },
            "stripe-webhook": { parent: "platform", name: "stripe-webhook" },
            "github-private-key": { parent: "platform", name: "github-private-key" },
        },
        installations: {
            vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
            connection: install(
                connectionPackage,
                { credentials: "connections" },
                { isolation: "host", name: CONNECTION_INSTALLATION },
            ),
            forge: install(
                forgePackage,
                {
                    repositories: "repositories",
                    credentials: "credentials",
                    "github-webhook": "github-webhook",
                    "github-private-key": "github-private-key",
                },
                { isolation: "host", name: FORGE_INSTALLATION },
            ),
            finance: install(
                financePackage,
                { stripe: "stripe", "stripe-webhook": "stripe-webhook" },
                { isolation: "host", name: FINANCE_INSTALLATION },
            ),
            message: install(messagePackage, {}, { isolation: "host", name: "message" }),
            notification: install(
                notificationPackage,
                {},
                { isolation: "host", name: "notification" },
            ),
            relay: install(relayPackage, {}, { name: "relay", selection: { kind: "host" } }),
            home: install(
                blankPackage,
                {},
                { name: HOME_APPLICATION, selection: { kind: "host" } },
            ),
            ...own,
            social: install(socialPackage, {}, { isolation: "host", name: "social" }),
            observability: install(
                observabilityPackage,
                {},
                { isolation: "host", name: "observability" },
            ),
        },
    });
}

/** The stack of the European Union's home, the platform residency's, with the site. */
export const RESIDENCY_STACK_EU = residencyStack({
    site: install(sitePackage, {}, { name: "site", selection: { kind: "host" } }),
});

/** The stack of the United States' home. */
export const RESIDENCY_STACK_US = residencyStack({});
