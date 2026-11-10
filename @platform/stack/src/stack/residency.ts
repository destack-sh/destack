import { DOMAINS, UNIVERSE_PAGES } from "@destack/account/address";
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
import socialPackage from "@destack/social/package";
import { defineStack, type StackInput } from "@destack/stack";
import homePackage from "@destack/home/package";
import sitePackage from "@platform/site/package";
import { HOME_APPLICATION } from "@destack/account/object";
import { defineVault } from "@destack/vault/declare";
import vaultPackage from "@destack/vault/package";
import { CREDIT_PRICES, PLANS } from "@platform/pricing";
import { PLATFORM } from "../platform/platform.ts";

/** The platform account's vault in a residency's home, whose secrets the operator writes once. */
export const platformVault = defineVault({ name: "platform", spec: {} });

/** Route a hostname's every path to the relay's names. */
function relayRoute(hostname: string): NonNullable<StackInput["routes"]>[string] {
    return {
        hostname,
        path: "/",
        match: "prefix",
        installation: "relay",
        rules: [{ entry: { kind: "adapter", adapter: "names" } }],
    };
}

/** Declare a residency home's stack: the registry, billing, its relay, and the edge it serves. */
function residencyStack(edge: {
    readonly routes: NonNullable<StackInput["routes"]>;
    readonly installations: NonNullable<StackInput["installations"]>;
}) {
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
                relay: install(relayPackage, {}, { name: "relay" }),
                home: install(homePackage, {}, { name: HOME_APPLICATION }),
                ...edge.installations,
                social: install(socialPackage, {}, { isolation: "host", name: "social" }),
                observability: install(
                    observabilityPackage,
                    {},
                    { isolation: "host", name: "observability" },
                ),
            },
            routes: edge.routes,
        },
    );
}

/** Route one of the universe's pages to Home's view of the same name. */
function pageRoute(page: keyof typeof UNIVERSE_PAGES): NonNullable<StackInput["routes"]>[string] {
    return {
        hostname: DOMAINS.universe,
        path: UNIVERSE_PAGES[page],
        match: "prefix",
        installation: "home",
        rules: [{ entry: { kind: "view", view: UNIVERSE_PAGES[page].slice(1) } }],
    };
}

/** Route a relay's own host, and on the platform residency's home the edge and the site. */
function edgeOf(code: ResidencyCode) {
    const own = { [`relay-${code}`]: relayRoute(`relay-${code}.${DOMAINS.machine}`) };
    const edge = {
        spaces: relayRoute(`*.${DOMAINS.space}`),
        machines: relayRoute(`*.${DOMAINS.machine}`),
        // serve the universe's root and its pages from Home
        start: {
            hostname: DOMAINS.universe,
            path: "/",
            match: "prefix",
            installation: "home",
            rules: [{ entry: { kind: "view", view: "start" } }],
        },
        "sign-in": pageRoute("signIn"),
        "sign-up": pageRoute("signUp"),
        consent: pageRoute("consent"),
        verification: pageRoute("verification"),
        apps: pageRoute("apps"),
        // serve the site's static pages at its own domain
        site: {
            hostname: DOMAINS.site,
            path: "/",
            match: "prefix",
            installation: "site",
            rules: [{ entry: { kind: "view", view: "site" } }],
        },
    } satisfies NonNullable<StackInput["routes"]>;

    return code === PLATFORM.residencyId
        ? {
              routes: { ...own, ...edge },
              installations: { site: install(sitePackage, {}, { name: "site" }) },
          }
        : { routes: own, installations: {} };
}

/** The stack of the European Union's home, the platform residency's, taking the edge's names. */
export const RESIDENCY_STACK_EU = residencyStack(edgeOf("eu"));

/** The stack of the United States' home. */
export const RESIDENCY_STACK_US = residencyStack(edgeOf("us"));
