import { DOMAINS } from "@destack/account/address";
import type { ResidencyCode } from "@destack/account/object";
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
import { defineStack } from "@destack/stack";
import { defineVault } from "@destack/vault/declare";
import vaultPackage from "@destack/vault/package";
import { CREDIT_PRICES, PLANS } from "@platform/pricing";
import { PLATFORM } from "../platform/platform.ts";

/** The platform account's vault in a residency's home, whose secrets the operator writes once. */
export const platformVault = defineVault({ name: "platform", spec: {} });

/** Route a hostname's every path to the relay's names. */
function relayRoute(hostname: string) {
    return {
        hostname,
        path: "/",
        match: "prefix",
        installation: "relay",
        adapter: "names",
    } as const;
}

/** Declare a residency home's stack: the registry, billing, and its relay at some routes. */
function residencyStack(routes: Readonly<Record<string, ReturnType<typeof relayRoute>>>) {
    return defineStack(
        [financePackage, vaultPackage, connectionPackage, forgePackage, gitStorage],
        {
            sellers: { platform: { provider: "stripe", creditPrices: CREDIT_PRICES } },
            ...defineCatalog(PLANS),
            vaults: {
                platform: platformVault.declare(),
                credentials: credentialVault.declare(),
                connections: connectionVault.declare(),
            },
            // the platform's Stripe account, whose key the deploy rotates in
            connections: {
                stripe: {
                    ...stripe.declare(),
                    protocol: { kind: "key", header: "authorization", scheme: "Bearer" },
                },
            },
            gitStorages: { repositories: repositoryStorage.declare() },
            issuers: { github: {} },
            secrets: {
                "github-webhook": { parent: "platform", name: "github-webhook" },
                "github-private-key": { parent: "platform", name: "github-private-key" },
            },
            installations: {
                vault: install(vaultPackage, {}, { isolation: "process", name: "vault" }),
                connection: install(
                    connectionPackage,
                    { credentials: "connections" },
                    { isolation: "process", name: CONNECTION_INSTALLATION },
                ),
                forge: install(
                    forgePackage,
                    {
                        repositories: "repositories",
                        credentials: "credentials",
                        "github-webhook": "github-webhook",
                        "github-private-key": "github-private-key",
                    },
                    { isolation: "process", name: FORGE_INSTALLATION },
                ),
                finance: install(
                    financePackage,
                    { stripe: "stripe" },
                    { isolation: "process", name: FINANCE_INSTALLATION },
                ),
                message: install(messagePackage, {}, { isolation: "process", name: "message" }),
                notification: install(
                    notificationPackage,
                    {},
                    { isolation: "process", name: "notification" },
                ),
                relay: install(relayPackage, {}, { name: "relay" }),
                observability: install(
                    observabilityPackage,
                    {},
                    { isolation: "process", name: "observability" },
                ),
            },
            routes,
        },
    );
}

/** Route a relay's own host, and on the platform residency's home every edge name. */
function edgeRoutes(code: ResidencyCode) {
    const own = { [`relay-${code}`]: relayRoute(`relay-${code}.${DOMAINS.machine}`) };
    const edge = {
        spaces: relayRoute(`*.${DOMAINS.space}`),
        machines: relayRoute(`*.${DOMAINS.machine}`),
    };

    return code === PLATFORM.residencyId ? { ...own, ...edge } : own;
}

/** The stack of a residency's home reached through no public names, as in development. */
export const RESIDENCY_STACK = residencyStack({});

/** The stack of the European Union's home, the platform residency's, taking the edge's names. */
export const RESIDENCY_STACK_EU = residencyStack(edgeRoutes("eu"));

/** The stack of the United States' home. */
export const RESIDENCY_STACK_US = residencyStack(edgeRoutes("us"));
