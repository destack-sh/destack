import { DeclarationReference } from "@destack/package/declare";
import * as relay from "@destack/relay/declare";
import { present } from "@destack/schema";
import { UniverseDefinition } from "@destack/universe";
import { expect, test } from "@destack/test";
import { RESIDENCY_STACK, STACKS } from "@destack/account/test";
import type { StackRelease } from "@destack/account/object";

/** A build answering the account test fixture's stacks, and its stand-in stack for every home. */
const BUILD = { stacks: () => STACKS, released: () => RESIDENCY_STACK };

/** The ports of the default local universe. */
const PORTS = LocalPorts.at(4100);

/** The local universe's address, as its homes' relays read it. */
const LOCAL = {
    origin: "http://destack.localhost:4100",
    domains: { space: "space.localhost", machine: "computer.localhost", site: "site.localhost" },
};

/** The production universe's address, as its homes' relays read it. */
const PRODUCTION = {
    origin: "https://destack.app",
    domains: { space: "destack.space", machine: "destack.computer", site: "destack.sh" },
};
import { provider } from "@destack/finance/declare";
import { PLATFORM, RESIDENCIES } from "@platform/stack";
import { ProviderId } from "@destack/account/authentication";
import { Custody, Deployment, LocalPorts } from "./index.ts";
import { type ScriptTarget, WorkerScript } from "../cloudflare/script.ts";
import { testSimulation } from "@destack/test/world";

/** List a universe's domains and each residency's hostnames, the public ones at the platform's. */
function names(domain: string, space: string, machine: string, site: string) {
    return [
        [space, machine, domain, site],
        {
            eu: [`relay-eu.${machine}`, `*.${space}`, `*.${machine}`, domain, site],
            us: [`relay-us.${machine}`],
        },
    ];
}

test("declare each universe's universe with its residencies, the reserved handles and a cloud machine per residency at its origin under the key its custody keeps", async () => {
    // keep each machine's key in a local custody record
    const host = testSimulation().environment("deployment");
    const custody = await Custody.local(
        RESIDENCIES.map((residency) => residency.machine.id),
        host,
    );

    // declare the dev and the production universe
    const local = Deployment.local(PORTS, host).definition("operator@destack.test", custody, BUILD);
    const production = Deployment.cloudflare("production", host).definition(
        "ops@destack.test",
        custody,
        BUILD,
    );
    const keys = RESIDENCIES.map((residency) => custody.machine(residency.machine.id).publicKey);

    // declare a valid universe with the machines at their loopback ports locally and at their one-label names in production
    expect({
        isValid: UniverseDefinition.safeParse(production).success,
        residencies: production.residencies,
        reserved: production.platform.reserved,
        local: local.platform.machines.map(({ name, origin }) => [name, origin]),
        production: production.platform.machines.map(({ name, origin, key }) => [
            name,
            origin,
            key,
        ]),
        names: {
            local: [local.platform.domains, local.platform.hostnames],
            production: [production.platform.domains, production.platform.hostnames],
        },
    }).toEqual({
        isValid: true,
        residencies: [
            {
                code: "eu",
                name: "European Union",
                relay: "https://relay-eu.destack.computer",
                countries: [
                    "AT",
                    "BE",
                    "BG",
                    "CH",
                    "CY",
                    "CZ",
                    "DE",
                    "DK",
                    "EE",
                    "ES",
                    "FI",
                    "FR",
                    "GB",
                    "GR",
                    "HR",
                    "HU",
                    "IE",
                    "IS",
                    "IT",
                    "LI",
                    "LT",
                    "LU",
                    "LV",
                    "MT",
                    "NL",
                    "NO",
                    "PL",
                    "PT",
                    "RO",
                    "SE",
                    "SI",
                    "SK",
                ],
            },
            {
                code: "us",
                name: "United States",
                relay: "https://relay-us.destack.computer",
                countries: ["US"],
            },
        ],
        reserved: PLATFORM.reserved,
        local: [
            ["eu-central", "http://127.0.0.1:4100"],
            ["us-east", "http://127.0.0.1:4102"],
        ],
        production: [
            ["eu-central", "https://eu-central--platform.destack.computer", keys[0]],
            ["us-east", "https://us-east--platform.destack.computer", keys[1]],
        ],
        names: {
            local: names(
                "destack.localhost",
                "space.localhost",
                "computer.localhost",
                "site.localhost",
            ),
            production: names("destack.app", "destack.space", "destack.computer", "destack.sh"),
        },
    });
});

test("select each residency home's stack release with its relay's origin, residency and universe, and its hostnames' routes", async () => {
    // select the homes of both universes over a build answering each export with what it places
    const host = testSimulation().environment("deployment");
    const build = {
        released: (exported: string, placed: Pick<StackRelease, "settings" | "routes">) => ({
            ...RESIDENCY_STACK,
            export: exported,
            ...placed,
        }),
    };
    const deployments = {
        local: Deployment.local(PORTS, host),
        production: Deployment.cloudflare("production", host),
    };
    const read = async (universe: "local" | "production") =>
        Object.fromEntries(
            Object.entries(deployments[universe].homes(build)).map(([code, home]) => [
                code,
                [
                    home.export,
                    home.settings[DeclarationReference.key(relay.origin.reference)],
                    home.settings[DeclarationReference.key(relay.residency.reference)],
                    home.settings[DeclarationReference.key(relay.universe.reference)],
                    Object.values(home.routes).map((route) => route["hostname"]),
                ],
            ]),
        );

    expect({ local: await read("local"), production: await read("production") }).toEqual({
        local: {
            eu: [
                "RESIDENCY_STACK_EU",
                "http://relay-eu.computer.localhost:4100",
                "eu",
                LOCAL,
                [
                    "relay-eu.computer.localhost",
                    "*.space.localhost",
                    "*.computer.localhost",
                    "destack.localhost",
                    "destack.localhost",
                    "site.localhost",
                ],
            ],
            us: [
                "RESIDENCY_STACK_US",
                "http://relay-us.computer.localhost:4102",
                "us",
                LOCAL,
                ["relay-us.computer.localhost"],
            ],
        },
        production: {
            eu: [
                "RESIDENCY_STACK_EU",
                "https://relay-eu.destack.computer",
                "eu",
                PRODUCTION,
                [
                    "relay-eu.destack.computer",
                    "*.destack.space",
                    "*.destack.computer",
                    "destack.app",
                    "destack.app",
                    "destack.sh",
                ],
            ],
            us: [
                "RESIDENCY_STACK_US",
                "https://relay-us.destack.computer",
                "us",
                PRODUCTION,
                ["relay-us.destack.computer"],
            ],
        },
    });
});

test("refuse the local universe where a deployment is asked for", () => {
    const host = testSimulation().environment("deployment");
    expect(() => Deployment.cloudflare("local", host)).toThrow(
        new Error("universe 'local' is not deployed: use 'production'"),
    );
});

test("charge through Stripe in production alone", () => {
    // read each universe's payment provider
    const host = testSimulation().environment("deployment");
    const providers = [
        Deployment.local(PORTS, host),
        Deployment.cloudflare("production", host),
    ].map((deployment) => deployment.settings[DeclarationReference.key(provider.reference)]);

    expect({ providers }).toEqual({
        providers: ["none", "stripe"],
    });
});

test("sign people in through GitHub, Google and Microsoft with each secret a universe holds, binding them on the universe's script alone", async () => {
    // read the applications with every secret, and with none
    const host = testSimulation().environment("deployment");
    const custody = await Custody.local(
        RESIDENCIES.map((residency) => residency.machine.id),
        host,
    );
    const production = Deployment.cloudflare("production", host);
    const providers = production.providers((id) => `${id}-secret`);
    const access = { accessKeyId: "worker", secretAccessKey: "worker-secret" };

    // bind them on the universe's script, and on no machine's
    const bound = (target: ScriptTarget) =>
        new WorkerScript(production, target).secrets(custody, access, providers)[
            "DESTACK_PROVIDERS"
        ];
    const residency = present(RESIDENCIES[0], "a residency");

    expect({
        secrets: ["github", "google", "microsoft"].map((id) =>
            Deployment.providerSecret(ProviderId.parse(id)),
        ),
        none: Deployment.local(PORTS, host).providers(() => undefined),
        every: providers,
        universe: bound({ kind: "universe" }),
        machine: bound({ kind: "machine", residency }) ?? null,
    }).toEqual({
        secrets: ["GH_CLIENT_SECRET", "GOOGLE_CLIENT_SECRET", "MICROSOFT_CLIENT_SECRET"],
        none: {},
        every: {
            github: { clientId: "Iv23li2jZs0kDg8Bc3sg", clientSecret: "github-secret" },
            google: {
                clientId:
                    "969772309074-tqcvgqsuj4mt3u6au8t2vbonmch2f40t.apps.googleusercontent.com",
                clientSecret: "google-secret",
            },
            microsoft: {
                clientId: "13d4b01a-bf0e-4ede-8fa6-1fcced42ad05",
                clientSecret: "microsoft-secret",
            },
        },
        universe: JSON.stringify(providers),
        machine: null,
    });
});
