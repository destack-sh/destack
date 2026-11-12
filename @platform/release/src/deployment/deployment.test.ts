import { DeclarationReference } from "@destack/package/declare";
import * as relay from "@destack/relay/declare";
import { type JsonValue, present } from "@destack/schema";
import { UniverseDefinition } from "@destack/control";
import { expect, test } from "@destack/test";
import { RESIDENCY_STACK, STACKS } from "@destack/account/test";

/** The stand-in stack of each residency's home. */
const HOMES = { eu: RESIDENCY_STACK, us: RESIDENCY_STACK };
import { provider } from "@destack/finance/declare";
import { PLATFORM, RESIDENCIES } from "@platform/stack";
import { ProviderId } from "@destack/account/authentication";
import { Custody, Deployment } from "./index.ts";
import { type ScriptTarget, WorkerScript } from "./cloudflare/script.ts";
import { testSimulation } from "@destack/test/world";

test("declare each stage's universe with its residencies, the reserved handles and a cloud machine per residency at its origin under the key its custody keeps", async () => {
    // keep each machine's key in a local custody record
    const host = testSimulation().environment("deployment");
    const custody = await Custody.local(
        RESIDENCIES.map((residency) => residency.machine.id),
        host,
    );

    // declare the dev and the production universe
    const local = await Deployment.of("local", host).universe(
        "operator@destack.test",
        custody,
        STACKS,
        HOMES,
    );
    const production = await Deployment.of("production", host).universe(
        "ops@destack.test",
        custody,
        STACKS,
        HOMES,
    );
    const keys = RESIDENCIES.map((residency) => custody.machine(residency.machine.id).publicKey);

    // declare the same names on every stage, the public ones at the platform residency
    const names = [
        ["destack.space", "destack.computer", "destack.app", "destack.sh"],
        {
            eu: [
                "relay-eu.destack.computer",
                "*.destack.space",
                "*.destack.computer",
                "destack.app",
                "destack.sh",
            ],
            us: ["relay-us.destack.computer"],
        },
    ];

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
            ["eu-central", "http://127.0.0.1:4102"],
            ["us-east", "http://127.0.0.1:4103"],
        ],
        production: [
            ["eu-central", "https://eu-central--platform.destack.computer", keys[0]],
            ["us-east", "https://us-east--platform.destack.computer", keys[1]],
        ],
        names: {
            local: names,
            production: names,
        },
    });
});

test("select each residency home's stack release with its relay's origin, residency and issuer, the edge's routes in production alone", async () => {
    // select the homes of both stages over a build answering each export with its settings
    const host = testSimulation().environment("deployment");
    const build = {
        released: (exported: string, settings: Readonly<Record<string, JsonValue>>) => ({
            ...RESIDENCY_STACK,
            export: exported,
            settings: { ...settings },
        }),
    };
    const read = async (stage: "local" | "production") =>
        Object.fromEntries(
            Object.entries(await Deployment.of(stage, host).homes(build)).map(([code, home]) => [
                code,
                [
                    home.export,
                    home.settings[DeclarationReference.key(relay.origin.reference)],
                    home.settings[DeclarationReference.key(relay.residency.reference)],
                    home.settings[DeclarationReference.key(relay.issuer.reference)],
                ],
            ]),
        );

    expect({ local: await read("local"), production: await read("production") }).toEqual({
        local: {
            eu: [
                "RESIDENCY_STACK_EU",
                "http://127.0.0.1:4102",
                "eu",
                "http://127.0.0.1:4100/oauth",
            ],
            us: [
                "RESIDENCY_STACK_US",
                "http://127.0.0.1:4103",
                "us",
                "http://127.0.0.1:4100/oauth",
            ],
        },
        production: {
            eu: [
                "RESIDENCY_STACK_EU",
                "https://relay-eu.destack.computer",
                "eu",
                "https://destack.app/oauth",
            ],
            us: [
                "RESIDENCY_STACK_US",
                "https://relay-us.destack.computer",
                "us",
                "https://destack.app/oauth",
            ],
        },
    });
});

test("refuse the local stage where a deployment is asked for", () => {
    const host = testSimulation().environment("deployment");
    expect(() => Deployment.deployed("local", host)).toThrow(
        new Error("the local stage runs under just local and deploys nothing"),
    );
});

test("charge through Stripe in production alone", () => {
    // read each stage's payment provider
    const host = testSimulation().environment("deployment");
    const providers = (["local", "production"] as const).map(
        (name) => Deployment.of(name, host).settings[DeclarationReference.key(provider.reference)],
    );

    expect({ providers }).toEqual({
        providers: ["none", "stripe"],
    });
});

test("sign people in through GitHub, Google and Microsoft with each secret a stage holds, binding them on the universe's script alone", async () => {
    // read the applications with every secret, and with none
    const host = testSimulation().environment("deployment");
    const custody = await Custody.local(
        RESIDENCIES.map((residency) => residency.machine.id),
        host,
    );
    const production = Deployment.of("production", host);
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
        none: Deployment.of("local", host).providers(() => undefined),
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
