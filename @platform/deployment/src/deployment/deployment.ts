import { TelemetryScope } from "@destack/telemetry";
import { DeclarationReference } from "@destack/package/declare";
import { UNIT_SKUS } from "@platform/pricing";
import type { RegionCode, StackRelease } from "@destack/account/object";
import { UniverseOrigin } from "@destack/account/address";
import { ProviderId } from "@destack/account/authentication";
import type { ProviderClients } from "@destack/account/better-auth";
import type { PlatformDefinition } from "@destack/account/server";
import type { Jurisdiction } from "@destack/db/cloudflare";
import type { Environment, HostCapability } from "@destack/environment";
import type { LogFormat } from "@destack/environment/line";
import { provider, tag } from "@destack/finance/declare";
import type { MailSender } from "@destack/mail/sender";
import { LATEST_TAG } from "@destack/registry/object";
import { type JsonValue, present } from "@destack/schema";
import type { UniverseDefinition } from "@destack/universe";
import type { PlatformBuild } from "@platform/distribution";
import { PLATFORM, type PlatformRegion, REGIONS } from "@platform/stack";
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

/** The provider running the managed machines. */
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
    /** The universe's port, which the platform region's managed machine serves. */
    readonly universe: number;
    /** This computer's own machine's port, which the local universe's people use. */
    readonly machine: number;
    /** Each region's managed machine's port, by region code. */
    readonly machines: Readonly<Record<RegionCode, number>>;
    /** The port wrangler serves the universe's Worker on under workerd. */
    readonly worker: number;
    /** The port the local release client's OIDC issuer answers on. */
    readonly publisher: number;
}

/** The loopback ports of local universes. */
export const LocalPorts = {
    /** Lay a local universe's ports out from its universe port, the other services after it. */
    at(universe: number): LocalPorts {
        // serve the platform region at the universe's port, and each other region after it
        const others = REGIONS.filter((region) => region.code !== PLATFORM.regionId);
        const machines: Record<RegionCode, number> = Object.fromEntries([
            [PLATFORM.regionId, universe] as const,
            ...others.map((region, index) => [region.code, universe + 2 + index] as const),
        ]);

        // put the Worker after the regions, and the release client's issuer after it
        const worker = universe + 2 + others.length;

        return { universe, machine: universe + 1, machines, worker, publisher: worker + 1 };
    },
};

/** The platform deployed at a universe: its origins, its mail and the settings its home places. */
export class Deployment {
    /** The universe's name, which names its roots, scripts, custody record and state. */
    readonly name: string;
    /** The universe's address: its origin, such as https://destack.app, and its domains. */
    readonly universe: UniverseOrigin;
    /** The distribution tag the first-party releases are published at. */
    readonly tag: string;
    /** Each region's managed machine's origin, by region code. */
    readonly machines: Readonly<Record<RegionCode, string>>;
    /** Each region's relay origin, where machines open their tunnels, by region code. */
    readonly relays: Readonly<Record<RegionCode, string>>;
    /** The jurisdiction keeping each region's objects and files, by region code. */
    readonly jurisdictions: Readonly<Record<RegionCode, Jurisdiction>>;
    /** Where the universe's Workers send mail. */
    readonly mail: MailSender;
    /** The setting values the platform home's stack places, by setting key. */
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

        // serve at the platform's domains, each region in its own jurisdiction
        const universe = new UniverseOrigin(PRODUCTION_ORIGIN, PRODUCTION_DOMAINS);
        const { machine } = universe.domains;

        return new Deployment(
            {
                name,
                universe,
                tag: LATEST_TAG,
                machines: Deployment.#regions((region) =>
                    universe.url(`${region.machine.name}--platform.${machine}`),
                ),
                relays: Deployment.#regions((region) =>
                    universe.url(`relay-${region.code}.${machine}`),
                ),
                jurisdictions: Deployment.#regions((region) => region.jurisdiction),
                mail: { kind: "ses", region: MAIL_REGION, from: MAIL_FROM },
                settings: Deployment.#settings(LATEST_TAG, "stripe"),
                logFormat: "json",
            },
            environment,
        );
    }

    /** Read the deployment of the local universe listening on loopback ports. */
    static local(ports: LocalPorts, environment: Environment<HostCapability>): Deployment {
        // serve on loopback, each relay at its machine's port, no region in a jurisdiction
        const port = (region: PlatformRegion) =>
            present(ports.machines[region.code], `the port of ${region.code}`);

        return new Deployment(
            {
                name: LOCAL,
                universe: new UniverseOrigin(
                    `http://${LOCAL_DOMAIN}:${ports.universe}`,
                    LOCAL_DOMAINS,
                ),
                tag: LOCAL,
                machines: Deployment.#regions((region) => `http://${LOOPBACK}:${port(region)}`),
                relays: Deployment.#regions(
                    (region) =>
                        `http://relay-${region.code}.${LOCAL_DOMAINS.machine}:${port(region)}`,
                ),
                jurisdictions: Deployment.#regions(() => "none"),
                mail: { kind: "print" },
                settings: Deployment.#settings(LOCAL, "none"),
                logFormat: "line",
            },
            environment,
        );
    }

    /** The origin the managed machine serving a region answers at. */
    machineOrigin(region: PlatformRegion): string {
        return present(this.machines[region.code], `the machine of ${region.code}`);
    }

    /** The origin machines open their tunnels at to a region's relay. */
    relayOrigin(region: PlatformRegion): string {
        return present(this.relays[region.code], `the relay of ${region.code}`);
    }

    /** The jurisdiction keeping a region's objects and files. */
    jurisdiction(region: PlatformRegion): Jurisdiction {
        return present(this.jurisdictions[region.code], `the jurisdiction of ${region.code}`);
    }

    /** Select the platform home's stack, with its settings and its domains' routes. */
    home(build: Pick<PlatformBuild, "released">): StackRelease {
        // serve the universe's pages from Home, and the site at its own domain
        const { domain, domains } = this.universe;

        return build.released("PLATFORM_STACK", {
            settings: this.settings,
            routes: {
                start: Deployment.#startRoute(domain, "home"),
                home: Deployment.#originRoute(domain, "home"),
                site: Deployment.#viewRoute(domains.site, "site", "site"),
            },
        });
    }

    /** Route a hostname's root alone to an installation's start view, before its other paths. */
    static #startRoute(hostname: string, installation: string) {
        return {
            hostname,
            path: "/",
            match: "exact",
            installation,
            rules: [{ entry: { kind: "view", view: "start" } }],
        };
    }

    /** Route a hostname's every path to an installation's whole origin. */
    static #originRoute(hostname: string, installation: string) {
        return {
            hostname,
            path: "/",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "origin" } }],
        };
    }

    /** Route a hostname's every path to one view of an installation. */
    static #viewRoute(hostname: string, installation: string, view: string) {
        return {
            hostname,
            path: "/",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "view", view } }],
        };
    }

    /** Declare the universe with a build's stacks, its managed machines under custody. */
    definition(
        operator: string,
        custody: Custody,
        build: Pick<PlatformBuild, "released" | "stacks">,
    ): UniverseDefinition {
        // declare each region's managed machine under the key its custody keeps, rating its spaces by the platform's SKUs
        const rates = Object.fromEntries(
            UNIT_SKUS.map((sku) => [sku.meterKey, DeclarationReference.key(sku.reference)]),
        );
        const machines = REGIONS.map((region) => ({
            id: region.machine.id,
            name: region.machine.name,
            regionId: region.code,
            providerCode: PROVIDER,
            location: region.location,
            key: custody.machine(region.machine.id).publicKey,
            origin: this.machineOrigin(region),
            rates,
        }));

        return {
            regions: REGIONS.map((region) => ({
                code: region.code,
                name: region.name,
                relay: this.relayOrigin(region),
                countries: [...region.countries],
            })),
            platform: {
                ...PLATFORM,
                operator,
                ...this.#domains(),
                home: this.home(build),
                stacks: build.stacks(),
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

    /** Map each region to a value. */
    static #regions<Value>(
        value: (region: PlatformRegion) => Value,
    ): Readonly<Record<RegionCode, Value>> {
        return Object.fromEntries(REGIONS.map((region) => [region.code, value(region)]));
    }

    /** List the settings the platform home's stack places: tag and payments provider. */
    static #settings(
        following: string,
        payments: "stripe" | "none",
    ): Readonly<Record<string, JsonValue>> {
        return {
            [DeclarationReference.key(tag.reference)]: following,
            [DeclarationReference.key(provider.reference)]: payments,
        };
    }

    /** Declare the platform's domains, and the hostnames its home's routes serve on. */
    #domains(): Pick<PlatformDefinition, "domains" | "hostnames"> {
        const { domain, domains } = this.universe;

        return {
            domains: [domains.space, domains.machine, domain, domains.site],
            hostnames: [domain, domains.site],
        };
    }
}
