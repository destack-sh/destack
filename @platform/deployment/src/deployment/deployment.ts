import { TelemetryScope } from "@destack/telemetry";
import { DeclarationReference } from "@destack/package/declare";
import { UNIT_SKUS } from "@platform/pricing";
import type { ResidencyCode, StackRelease } from "@destack/account/object";
import { UniverseOrigin } from "@destack/account/address";
import { ProviderId } from "@destack/account/authentication";
import type { ProviderClients } from "@destack/account/better-auth";
import type { PlatformDefinition } from "@destack/account/server";
import type { Jurisdiction } from "@destack/db/cloudflare";
import type { Environment, HostCapability, LogFormat } from "@destack/environment";
import { provider, tag } from "@destack/finance/declare";
import { github } from "@destack/forge/stack";
import type { MailSender } from "@destack/mail/sender";
import { LATEST_TAG } from "@destack/registry/object";
import * as relay from "@destack/relay/declare";
import { type JsonValue, present } from "@destack/schema";
import type { UniverseDefinition } from "@destack/universe";
import type { PlatformBuild } from "@platform/distribution";
import { GITHUB_APP, PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import type { Custody } from "./custody.ts";

/** The universe the platform deploys to Cloudflare. */
const PRODUCTION = "production";

/** The local universe, which runs on this computer and deploys nothing. */
const LOCAL = "local";

/** The loopback interface the local universe's processes listen on. */
export const LOOPBACK = "127.0.0.1";

/** The domains of the universe on Cloudflare beside its origin's. */
const PRODUCTION_DOMAINS = {
    space: "destack.space",
    machine: "destack.computer",
    site: "destack.sh",
};

/** The origin of the universe on Cloudflare. */
const PRODUCTION_ORIGIN = "https://destack.app";

/** The local universe's domain, which resolves to loopback by RFC 6761. */
const LOCAL_DOMAIN = "destack.localhost";

/** The local universe's domains beside its origin's, which resolve to loopback by RFC 6761. */
const LOCAL_DOMAINS = {
    space: "space.localhost",
    machine: "computer.localhost",
    site: "site.localhost",
};

/** The provider running the cloud machines. */
const PROVIDER = "cloudflare";

/** The variable holding each sign-in provider's client secret, GitHub's as Actions allows. */
const PROVIDER_SECRETS: Readonly<Record<ProviderId, string>> = {
    github: "GH_CLIENT_SECRET",
    google: "GOOGLE_CLIENT_SECRET",
    microsoft: "MICROSOFT_CLIENT_SECRET",
};

/** The client identifier of Destack's application at each sign-in provider, for every universe. */
const PROVIDER_CLIENT_IDS: Readonly<Record<ProviderId, string>> = {
    github: "Iv23li2jZs0kDg8Bc3sg",
    google: "969772309074-tqcvgqsuj4mt3u6au8t2vbonmch2f40t.apps.googleusercontent.com",
    microsoft: "13d4b01a-bf0e-4ede-8fa6-1fcced42ad05",
};

/** The AWS region SES sends Destack's mail from, where the global root verifies destack.app. */
const MAIL_REGION = "eu-central-2";

/** The mailbox Destack's mail comes from, on the domain SES verifies. */
const MAIL_FROM = "Destack <notices@destack.app>";

/** The loopback port each service of a local universe listens on. */
export interface LocalPorts {
    /** The universe's port, which the platform residency's cloud machine serves. */
    readonly universe: number;
    /** This computer's own machine's port, which the local universe's people use. */
    readonly machine: number;
    /** Each residency's cloud machine's port, by residency code. */
    readonly machines: Readonly<Record<ResidencyCode, number>>;
    /** The port wrangler serves the universe's Worker on under workerd. */
    readonly worker: number;
}

/** The loopback ports of local universes. */
export const LocalPorts = {
    /** Lay a local universe's ports out from its universe port, the other services after it. */
    at(universe: number): LocalPorts {
        // serve the platform residency at the universe's port, and each other residency after it
        const others = RESIDENCIES.filter((residency) => residency.code !== PLATFORM.residencyId);
        const machines: Record<ResidencyCode, number> = Object.fromEntries([
            [PLATFORM.residencyId, universe] as const,
            ...others.map((residency, index) => [residency.code, universe + 2 + index] as const),
        ]);

        return { universe, machine: universe + 1, machines, worker: universe + 2 + others.length };
    },
};

/** The platform deployed at a universe: its origins, its mail and the settings its homes place. */
export class Deployment {
    /** The universe's name, which names its roots, scripts, custody record and state. */
    readonly name: string;
    /** The universe's address: its origin, such as https://destack.app, and its domains. */
    readonly universe: UniverseOrigin;
    /** The distribution tag of the first-party releases the universe's spaces follow. */
    readonly tag: string;
    /** Each residency's cloud machine's origin, by residency code. */
    readonly machines: Readonly<Record<ResidencyCode, string>>;
    /** Each residency's relay origin, where machines open their tunnels, by residency code. */
    readonly relays: Readonly<Record<ResidencyCode, string>>;
    /** The jurisdiction keeping each residency's objects and files, by residency code. */
    readonly jurisdictions: Readonly<Record<ResidencyCode, Jurisdiction>>;
    /** Where the universe's Workers send mail. */
    readonly mail: MailSender;
    /** The setting values the residency homes' platform stack places, by setting key. */
    readonly settings: Readonly<Record<string, JsonValue>>;
    /** The format the universe's Workers print in. */
    readonly logFormat: LogFormat;
    /** The host the deployment's tools run on. */
    readonly environment: Environment<HostCapability>;
    /** The deployment's telemetry, whose records print as status lines. */
    readonly scope: TelemetryScope;

    /** Create a deployment from its values and its host. */
    private constructor(
        values: Pick<
            Deployment,
            | "name"
            | "universe"
            | "tag"
            | "machines"
            | "relays"
            | "jurisdictions"
            | "mail"
            | "settings"
            | "logFormat"
        >,
        environment: Environment<HostCapability>,
    ) {
        // keep the values, the host and its telemetry
        this.name = values.name;
        this.universe = values.universe;
        this.tag = values.tag;
        this.machines = values.machines;
        this.relays = values.relays;
        this.jurisdictions = values.jurisdictions;
        this.mail = values.mail;
        this.settings = values.settings;
        this.logFormat = values.logFormat;
        this.environment = environment;
        this.scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    }

    /** Read the deployment of a universe on Cloudflare by its name, refusing any other. */
    static cloudflare(
        name: string | undefined,
        environment: Environment<HostCapability>,
    ): Deployment {
        // refuse a universe the platform does not deploy
        if (name !== PRODUCTION) {
            throw new Error(`universe '${name}' is not deployed: use '${PRODUCTION}'`);
        }

        // serve at the platform's domains, each residency in its own jurisdiction
        const universe = new UniverseOrigin(PRODUCTION_ORIGIN, PRODUCTION_DOMAINS);
        const { machine } = universe.domains;

        return new Deployment(
            {
                name,
                universe,
                tag: LATEST_TAG,
                machines: Deployment.#residencies((residency) =>
                    universe.url(`${residency.machine.name}--platform.${machine}`),
                ),
                relays: Deployment.#residencies((residency) =>
                    universe.url(`relay-${residency.code}.${machine}`),
                ),
                jurisdictions: Deployment.#residencies((residency) => residency.jurisdiction),
                mail: { kind: "ses", region: MAIL_REGION, from: MAIL_FROM },
                settings: Deployment.#settings(
                    LATEST_TAG,
                    { kind: "app", appId: GITHUB_APP },
                    "stripe",
                ),
                logFormat: "json",
            },
            environment,
        );
    }

    /** Read the deployment of the local universe listening on loopback ports. */
    static local(ports: LocalPorts, environment: Environment<HostCapability>): Deployment {
        // serve on loopback, each relay at its machine's port, no residency in a jurisdiction
        const port = (residency: PlatformResidency) =>
            present(ports.machines[residency.code], `the port of ${residency.code}`);

        return new Deployment(
            {
                name: LOCAL,
                universe: new UniverseOrigin(
                    `http://${LOCAL_DOMAIN}:${ports.universe}`,
                    LOCAL_DOMAINS,
                ),
                tag: LOCAL,
                machines: Deployment.#residencies(
                    (residency) => `http://${LOOPBACK}:${port(residency)}`,
                ),
                relays: Deployment.#residencies(
                    (residency) =>
                        `http://relay-${residency.code}.${LOCAL_DOMAINS.machine}:${port(residency)}`,
                ),
                jurisdictions: Deployment.#residencies(() => "none"),
                mail: { kind: "print" },
                settings: Deployment.#settings(LOCAL, { kind: "none" }, "none"),
                logFormat: "line",
            },
            environment,
        );
    }

    /** The origin the cloud machine serving a residency answers at. */
    machineOrigin(residency: PlatformResidency): string {
        return present(this.machines[residency.code], `the machine of ${residency.code}`);
    }

    /** The origin machines open their tunnels at to a residency's relay. */
    relayOrigin(residency: PlatformResidency): string {
        return present(this.relays[residency.code], `the relay of ${residency.code}`);
    }

    /** The jurisdiction keeping a residency's objects and files. */
    jurisdiction(residency: PlatformResidency): Jurisdiction {
        return present(this.jurisdictions[residency.code], `the jurisdiction of ${residency.code}`);
    }

    /** Select each residency home's stack with its relay's settings and its hostnames' routes. */
    homes(build: Pick<PlatformBuild, "released">): Record<string, StackRelease> {
        // serve the universe's root from Home's start, and the site's pages at its own domain
        const { domain, domains } = this.universe;
        const platform = {
            spaces: Deployment.#relayRoute(`*.${domains.space}`),
            machines: Deployment.#relayRoute(`*.${domains.machine}`),
            start: Deployment.#viewRoute(domain, "home", "start"),
            site: Deployment.#viewRoute(domains.site, "site", "site"),
        };

        // place each home's relay settings and its relay's route, the platform's routes in the platform residency's home
        const homes: Record<string, StackRelease> = {};
        for (const residency of RESIDENCIES) {
            const host = new URL(this.relayOrigin(residency)).hostname;
            homes[residency.code] = build.released(residency.stack, {
                settings: {
                    ...this.settings,
                    [DeclarationReference.key(relay.origin.reference)]: this.relayOrigin(residency),
                    [DeclarationReference.key(relay.residency.reference)]: residency.code,
                    [DeclarationReference.key(relay.universe.reference)]: this.universe.toJSON(),
                },
                routes: {
                    [`relay-${residency.code}`]: Deployment.#relayRoute(host),
                    ...(residency.code === PLATFORM.residencyId ? platform : {}),
                },
            });
        }

        return homes;
    }

    /** Route a hostname's every path to a view of an installation. */
    static #viewRoute(hostname: string, installation: string, view: string) {
        return {
            hostname,
            path: "/",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "view", view } }],
        };
    }

    /** Route a hostname's every path to the relay's names. */
    static #relayRoute(hostname: string) {
        return {
            hostname,
            path: "/",
            match: "prefix",
            installation: "relay",
            rules: [{ entry: { kind: "mount", mount: "names" } }],
        };
    }

    /** Declare the universe with a build's stacks, its cloud machines under custody. */
    definition(
        operator: string,
        custody: Custody,
        build: Pick<PlatformBuild, "released" | "stacks">,
    ): UniverseDefinition {
        // declare each residency's cloud machine under the key its custody keeps, rating its spaces by the platform's SKUs
        const rates = Object.fromEntries(
            UNIT_SKUS.map((sku) => [sku.meterKey, DeclarationReference.key(sku.reference)]),
        );
        const machines = RESIDENCIES.map((residency) => ({
            id: residency.machine.id,
            name: residency.machine.name,
            residencyId: residency.code,
            providerCode: PROVIDER,
            location: residency.location,
            key: custody.machine(residency.machine.id).publicKey,
            origin: this.machineOrigin(residency),
            rates,
        }));

        return {
            residencies: RESIDENCIES.map((residency) => ({
                code: residency.code,
                name: residency.name,
                relay: this.relayOrigin(residency),
                countries: [...residency.countries],
            })),
            platform: {
                ...PLATFORM,
                operator,
                ...this.#domains(),
                homes: this.homes(build),
                stacks: build.stacks(this.tag),
                machines,
            },
        };
    }

    /** Read the applications at the sign-in providers, leaving out one without a secret. */
    providers(secret: (id: ProviderId) => string | undefined): ProviderClients {
        return Object.fromEntries(
            ProviderId.options.flatMap((id) => {
                const clientSecret = secret(id);

                return clientSecret === undefined
                    ? []
                    : [[id, { clientId: PROVIDER_CLIENT_IDS[id], clientSecret }]];
            }),
        );
    }

    /** Name the variable a deploy reads a sign-in provider's client secret from. */
    static providerSecret(id: ProviderId): string {
        return PROVIDER_SECRETS[id];
    }

    /** Map each residency to a value. */
    static #residencies<Value>(
        value: (residency: PlatformResidency) => Value,
    ): Readonly<Record<ResidencyCode, Value>> {
        return Object.fromEntries(
            RESIDENCIES.map((residency) => [residency.code, value(residency)]),
        );
    }

    /** List the settings the homes' platform stack places: tag, GitHub and payments provider. */
    static #settings(
        following: string,
        app: JsonValue,
        payments: "stripe" | "none",
    ): Readonly<Record<string, JsonValue>> {
        return {
            [DeclarationReference.key(tag.reference)]: following,
            [DeclarationReference.key(github.reference)]: app,
            [DeclarationReference.key(provider.reference)]: payments,
        };
    }

    /** Declare the platform's domains and its residency accounts' hostnames. */
    #domains(): Pick<PlatformDefinition, "domains" | "hostnames"> {
        // route each relay's host at its residency, and the platform's domains at the platform residency
        const { domain, domains } = this.universe;
        const platform = [`*.${domains.space}`, `*.${domains.machine}`, domain, domains.site];
        const hostnames = RESIDENCIES.map((residency): [string, string[]] => [
            residency.code,
            [
                new URL(this.relayOrigin(residency)).hostname,
                ...(residency.code === PLATFORM.residencyId ? platform : []),
            ],
        ]);

        return {
            domains: [domains.space, domains.machine, domain, domains.site],
            hostnames: Object.fromEntries(hostnames),
        };
    }
}
