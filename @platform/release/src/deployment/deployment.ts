import { DeclarationReference } from "@destack/package/declare";
import type { ResidencyCode } from "@destack/account/object";
import { DOMAINS, UniverseOrigin } from "@destack/account/address";
import { ProviderId } from "@destack/account/authentication";
import type { ProviderClients } from "@destack/account/better-auth";
import type { Jurisdiction } from "@destack/db/cloudflare";
import { Stage } from "@destack/update/release";
import { Identifier, type JsonValue, present, schema } from "@destack/schema";
import type { UniverseDefinition } from "@destack/control";
import type { PlatformDefinition } from "@destack/account/server";
import type { StackRelease } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { IssuerCredential, MachineKey, Session } from "@destack/account/client";
import { Envelope } from "@destack/identity/aws";
import { MemoryKeychain } from "@destack/identity";
import { createClient } from "@destack/object/client";
import { provider, tag } from "@destack/finance/declare";
import { github } from "@destack/forge/stack";
import forgePackage, { FORGE_INSTALLATION } from "@destack/forge/package";
import type { MailSender } from "@destack/mail/sender";
import { Issuer } from "@destack/account/client";
import { Account, account } from "@destack/account/object";
import { Claim, SpaceMount } from "@destack/object";
import registryPackage from "@destack/registry/package";
import { Registry } from "@destack/registry/client";
import { type ForgeOpen, Publication } from "../local/publication.ts";
import { ActionsToken } from "../github/oidc.ts";
import { Operator } from "../local/operator.ts";
import { PrintedMail } from "../local/mail.ts";
import { Bookmark } from "@destack/object/client";
import { VERSION_HEADER } from "@destack/object";
import type { Fetch } from "@destack/object";
import * as relay from "@destack/relay/declare";
import { resolve } from "node:path";
import { PostgresPool } from "@destack/db/postgres";
import {
    type SuppliedModules,
    UNIVERSE_MODULE,
    WorkerScript,
    type WranglerConfiguration,
} from "./cloudflare/script.ts";
import {
    Root,
    ROOT,
    type ResidencyOutputs,
    type UniverseOutputs,
    type WorkerOutputs,
    WRANGLER_VARIABLES,
} from "./root.ts";
import { S3Server } from "@destack/bucket/s3";
import { LocalBucket } from "@destack/bucket/bun";
import { LocalProcess } from "../local/process.ts";
import { type LocalBuild, PlatformBuild } from "./build.ts";
import { GITHUB_APP, PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { Custody, type CustodyEnvelopes } from "./custody.ts";
import { DISTRIBUTION_MODULE, PackageId } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { SERVER_BUNDLE_MODULE } from "@destack/machine/cloudflare";
import { InstallationMount, PackageMount } from "@destack/object";
import type { Environment, HostCapability } from "@destack/environment";

/** How long a started machine's directory replica may take to sync: a minute. */
const DIRECTORY_SYNC_MILLISECONDS = 60_000;

/** How long a deployed residency home may take to serve: seconds from cached builds. */
const HOME_SERVE_MILLISECONDS = 30_000;

/** The identifier an end-to-end run binds where a deployment's state has a real one. */
const PLACEHOLDER_IDENTIFIER = "0".repeat(32);

/** The offset of a script's inspector port from its own port, clear of the loopback ports. */
const INSPECTOR_OFFSET = 5000;

/** The configuration keys an end-to-end run leaves out: routes, tails and artifacts. */
const DEPLOYED_KEYS = new Set(["routes", "tail_consumers", "artifacts"]);

/** The directory below a stage's target an end-to-end run plans its local versions in. */
const E2E_DIRECTORY = "e2e";

/** How often a check asks a space's readiness probe, as a probe is polled. */
const READY_POLL_MILLISECONDS = 1000;

/** A residency's cloud machine a check acts as: its sends toward packages, bound to a space. */
interface MachineActor {
    /** Send as the machine toward a package, bound to a space when given. */
    readonly fetch: (audience: PackageId, partition?: Identifier<"space">) => Fetch;
}

/** The password local Hyperdrive requires, which the trusting local server ignores. */
const HYPERDRIVE_PASSWORD = "local";

/** The line wrangler prints once local workerd serves a script. */
const READY_LINE = "Ready on";

/** The region an end-to-end run's files bucket signs for. */
const PLACEHOLDER_REGION = "local";

/** The repository's root, two levels above the release package. */
export const REPOSITORY = resolve(ROOT, "../..");

/** The access key a local run binds in place of a provisioned one. */
const PLACEHOLDER_KEY = { accessKeyId: "local", secretAccessKey: "local" };

/** The variable holding each sign-in provider's client secret, GitHub's as Actions allows. */
const PROVIDER_SECRETS: Readonly<Record<ProviderId, string>> = {
    github: "GH_CLIENT_SECRET",
    google: "GOOGLE_CLIENT_SECRET",
    microsoft: "MICROSOFT_CLIENT_SECRET",
};

/** The client identifier of Destack's application at each sign-in provider, for every stage. */
const PROVIDER_CLIENT_IDS: Readonly<Record<ProviderId, string>> = {
    github: "Iv23li2jZs0kDg8Bc3sg",
    google: "969772309074-tqcvgqsuj4mt3u6au8t2vbonmch2f40t.apps.googleusercontent.com",
    microsoft: "13d4b01a-bf0e-4ede-8fa6-1fcced42ad05",
};

/** The AWS region SES sends Destack's mail from, where the global root verifies destack.app. */
const MAIL_REGION = "eu-central-2";

/** The mailbox Destack's mail comes from, on the domain SES verifies. */
const MAIL_FROM = "Destack <notices@destack.app>";

/** What a local run's roots provision for a Worker, whose local custody envelopes nothing. */
const PLACEHOLDER_WORKER: WorkerOutputs = { envelope: { kind: "local" }, access: PLACEHOLDER_KEY };

/** The loopback interface the local stage's processes listen on. */
const LOOPBACK = "127.0.0.1";

/** The domain the cloud machines answer below, each one label as `<machine>--platform`. */
const MACHINE_DOMAIN = "destack.computer";

/** The provider running the cloud machines. */
const PROVIDER = "cloudflare";

/** The local universe's loopback port, which the stage's other ports follow. */
const UNIVERSE_PORT = Number(new URL(Stage.of("local").universe).port);

/** The loopback port each service of the local stage listens on. */
export interface LoopbackLayout {
    /** The universe's port, its issuer's. */
    readonly universe: number;
    /** This computer's own machine's port, which the local stage's people use. */
    readonly machine: number;
    /** Each residency's cloud machine's port, by residency code. */
    readonly machines: Readonly<Record<ResidencyCode, number>>;
}

/** The loopback layout of `just local`: the universe, this machine, then each residency's. */
export const LOOPBACK_LAYOUT: LoopbackLayout = {
    universe: UNIVERSE_PORT,
    machine: UNIVERSE_PORT + 1,
    machines: Object.fromEntries(
        RESIDENCIES.map((residency, index) => [residency.code, UNIVERSE_PORT + 2 + index]),
    ),
};

/** The platform deployed at a stage: its universe, residencies and Workers. */
export class Deployment {
    /** The stage deployed. */
    readonly stage: Stage;
    /** The loopback port each service of the local stage listens on. */
    readonly layout: LoopbackLayout;
    /** The host the deployment's tools run on. */
    readonly environment: Environment<HostCapability>;

    /** Deploy the platform at a stage from a host. */
    private constructor(
        stage: Stage,
        layout: LoopbackLayout,
        environment: Environment<HostCapability>,
    ) {
        this.stage = stage;
        this.layout = layout;
        this.environment = environment;
    }

    /** Read the deployment of a stage by its name, refusing any other. */
    static of(name: string | undefined, environment: Environment<HostCapability>): Deployment {
        return new Deployment(Stage.of(name), LOOPBACK_LAYOUT, environment);
    }

    /** Read the local stage's deployment listening on a loopback layout. */
    static loopback(layout: LoopbackLayout, environment: Environment<HostCapability>): Deployment {
        return new Deployment(Stage.of("local"), layout, environment);
    }

    /** Read a deployed stage's deployment by its name, refusing the local stage and any other. */
    static deployed(
        name: string | undefined,
        environment: Environment<HostCapability>,
    ): Deployment {
        const deployment = Deployment.of(name, environment);
        if (deployment.stage.name === "local") {
            throw new Error("the local stage runs under just local and deploys nothing");
        }

        return deployment;
    }

    /** The directory of the releases the Workers ship as their static assets, a build layer. */
    get releases(): string {
        return resolve(REPOSITORY, "target", this.stage.name, "cloud");
    }

    /** The origin of the universe the stage signs people in at. */
    get universeOrigin(): string {
        return this.stage.name === "local"
            ? `http://${LOOPBACK}:${this.layout.universe}`
            : this.stage.universe;
    }

    /** The origin the cloud machine serving a residency answers at. */
    machineOrigin(residency: PlatformResidency): string {
        // answer on its loopback port locally
        if (this.stage.name === "local") {
            const port = this.layout.machines[residency.code];

            return `http://${LOOPBACK}:${present(port, `the port of ${residency.code}`)}`;
        }

        return `https://${residency.machine.name}--platform.${MACHINE_DOMAIN}`;
    }

    /** The origin machines open their tunnels at to a residency's relay, its machine's locally. */
    relayOrigin(residency: PlatformResidency): string {
        return this.stage.name === "local"
            ? this.machineOrigin(residency)
            : `https://relay-${residency.code}.${DOMAINS.machine}`;
    }

    /** Pin each residency home's stack with its relay's settings and production's routes. */
    async homes(build: Pick<PlatformBuild, "pinned">): Promise<Record<string, StackRelease>> {
        const homes: Record<string, StackRelease> = {};
        for (const residency of RESIDENCIES) {
            homes[residency.code] = await build.pinned(residency.stack, {
                ...this.settings,
                [DeclarationReference.key(relay.origin.reference)]: this.relayOrigin(residency),
                [DeclarationReference.key(relay.residency.reference)]: residency.code,
                [DeclarationReference.key(relay.issuer.reference)]: this.issuer,
            });
        }

        return homes;
    }

    /** The jurisdiction keeping a residency's objects and files: none locally, as in workerd. */
    jurisdiction(residency: PlatformResidency): Jurisdiction {
        return this.stage.name === "local" ? "none" : residency.jurisdiction;
    }

    /** Where the stage's Workers send mail: through SES in production, else to their own logs. */
    get mail(): MailSender {
        return this.stage.name === "production"
            ? { kind: "ses", region: MAIL_REGION, from: MAIL_FROM }
            : { kind: "print" };
    }

    /** The setting values the residency homes' platform stack places, by setting key. */
    get settings(): Readonly<Record<string, JsonValue>> {
        return {
            [DeclarationReference.key(tag.reference)]: this.stage.tag,
            [DeclarationReference.key(github.reference)]:
                this.stage.name === "production"
                    ? { kind: "app", appId: GITHUB_APP }
                    : { kind: "none" },
            [DeclarationReference.key(provider.reference)]:
                this.stage.name === "production" ? "stripe" : "none",
        };
    }

    /** Declare the stage's universe. */
    async universe(
        operator: string,
        custody: Custody,
        stacks: PlatformDefinition["stacks"],
        homes: PlatformDefinition["homes"],
    ): Promise<UniverseDefinition> {
        // declare each residency's cloud machine under the key its custody keeps
        const machines = [];
        for (const residency of RESIDENCIES) {
            machines.push({
                id: residency.machine.id,
                name: residency.machine.name,
                residencyId: residency.code,
                providerCode: PROVIDER,
                location: residency.location,
                key: custody.machine(residency.machine.id).publicKey,
                origin: this.machineOrigin(residency),
            });
        }

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
                ...this.#edge(),
                homes,
                stacks,
                machines,
            },
        };
    }

    /** Serve the local universe and cloud machines under workerd, and check they answer. */
    async e2e(server: string, print: (name: string, line: string) => void): Promise<void> {
        // create a fresh database on the server's maintenance connection
        const { environment } = this;
        const database = `destack_e2e_${this.stage.name}`;
        const administration = new PostgresPool(`${server}/postgres`, environment, {
            connections: 1,
        });
        try {
            await administration.run(environment, async (sql) => {
                await sql.unsafe(`DROP DATABASE IF EXISTS "${database}" WITH (FORCE)`);
                await sql.unsafe(`CREATE DATABASE "${database}"`);
            });
        } finally {
            await administration.end();
        }

        // address the empty database
        const url = `${server}/${database}`;

        // connect local Hyperdrive with the password it requires
        const hyperdrive = new URL(url);
        hyperdrive.password ||= HYPERDRIVE_PASSWORD;

        // build the checkout at local prerelease versions, as the local stage does, as its working tree may be uncommitted
        const publication = await Publication.read(
            resolve(REPOSITORY, "target", this.stage.name, E2E_DIRECTORY),
            environment.files,
        );
        await using build = await this.#build(publication.build);

        // declare the universe over keys in the clear with the stacks built from the checkout
        const custody = await Custody.local(this.#machineIds(), environment);
        const declaration = await this.universe(
            Operator.LOCAL,
            custody,
            build.stacks(this.stage.tag),
            await this.homes(build),
        );

        // serve the universe's files from a local bucket over S3
        await using files = await LocalBucket.open(
            resolve(REPOSITORY, "target", this.stage.name, E2E_DIRECTORY, "files"),
            "universe",
            environment,
        );
        const s3 = new S3Server({
            region: PLACEHOLDER_REGION,
            credentials: async (id) =>
                id === PLACEHOLDER_KEY.accessKeyId ? PLACEHOLDER_KEY : undefined,
            open: async (name) => (name === this.#files ? files : undefined),
            environment,
        });
        const listener = environment.server.http({
            hostname: LOOPBACK,
            port: 0,
            fetch: (request) => s3.fetch(request),
        });
        await using serving = new AsyncDisposableStack();
        serving.defer(() => listener.stop(true));

        // configure the scripts over placeholder outputs
        const scripts = this.#scripts(this.#placeholderUniverse(listener.url), (residency) =>
            this.#placeholderResidency(residency),
        ).filter(([script]) => script.target.kind !== "tail");

        // write the releases the Workers ship as their static assets
        await build.writeAssets(this.releases);
        const modules = await Deployment.#modules(build, declaration);

        // serve each script on its own ports with placeholder secrets and its root key, and check it answers, keeping the universe's lines
        await using children = new AsyncDisposableStack();
        const mail = new PrintedMail();
        const placeholders = this.providers(() => PLACEHOLDER_KEY.secretAccessKey);
        for (const [script, configuration] of scripts) {
            // write the configuration without what only a deployment binds
            const local = Object.fromEntries(
                Object.entries(configuration).filter(([key]) => !DEPLOYED_KEYS.has(key)),
            );
            const path = await script.write(local, modules);
            const secrets = script.secrets(custody, PLACEHOLDER_KEY, placeholders);
            const variables = Object.entries(secrets).map(([name, value]) => `${name}=${value}\n`);
            await environment.files.writeTextFile(
                resolve(path, "..", ".dev.vars"),
                variables.join(""),
                { mode: 0o600 },
            );

            // start local workerd, waiting until it serves or exits
            const port = Number(new URL(script.origin).port);
            const ready = Promise.withResolvers<void>();
            const child = children.use(
                LocalProcess.start(
                    script.name,
                    {
                        command: [
                            resolve(ROOT, "node_modules/.bin/wrangler"),
                            "dev",
                            "--local",
                            "--config",
                            path,
                            "--ip",
                            LOOPBACK,
                            "--port",
                            String(port),
                            "--inspector-port",
                            String(port + INSPECTOR_OFFSET),
                        ],
                        directory: ROOT,
                        variables: {
                            ...Root.inherited(environment.process.variables, WRANGLER_VARIABLES),
                            CLOUDFLARE_HYPERDRIVE_LOCAL_CONNECTION_STRING_DATABASE: hyperdrive.href,
                            WRANGLER_SEND_METRICS: "false",
                        },
                        print: (name, line) => {
                            print(name, line);
                            if (script.target.kind === "universe") {
                                mail.push(line);
                            }
                            if (line.includes(READY_LINE)) {
                                ready.resolve();
                            }
                        },
                    },
                    environment,
                ),
            );
            const isServing = await Promise.race([
                ready.promise.then(() => true),
                child.exited.then(() => false),
            ]);
            if (!isServing) {
                throw new Error(`${script.name} exited before it served`);
            }

            // require it to answer at its origin
            await script.smoke();
            print("e2e", `${script.name} answers at ${script.origin}`);
        }

        // require each residency's home to serve its forge on the residency's cloud machine
        for (const residency of RESIDENCIES) {
            await this.#smokeForge(residency, custody, build);
            print("e2e", `the home of ${residency.code} serves its forge`);
        }

        // sign the operator in, and publish the platform's releases under the stage's tag as its owner
        const send = (request: Request) => this.environment.network.fetch(request);
        const issuer = new Issuer(this.issuer, send, this.environment, new Bookmark());
        const operator = await Operator.signIn(issuer, mail, this.environment);
        await Publication.release(
            build,
            this.stage.tag,
            this.forges(custody, build, operator),
            async () => {},
        );
        print("e2e", `the releases are published under ${this.stage.tag}`);

        // sign a person up in the first residency, as the CLI does, and require their home to serve
        const [first] = RESIDENCIES;
        const residency = present(first, "a residency");
        await this.#smokeSignUp(residency, custody, mail);
        print("e2e", `a person signed up in ${residency.code} and their home serves`);
    }

    /** Require a residency home's forge to serve on the residency's cloud machine. */
    async #smokeForge(
        residency: PlatformResidency,
        custody: Custody,
        build: PlatformBuild,
    ): Promise<void> {
        // read the residency's home at the universe as the machine, which may locate it
        const machine = await this.#actAs(residency, custody);
        const accounts = createClient(
            accountPackage,
            {
                url: PackageMount.url(this.universeOrigin, accountPackage.id),
                fetch: machine.fetch(accountPackage.id),
            },
            this.environment,
        );
        const { space } = await accounts.residency.get({ id: residency.code });
        const home = schema
            .identifier("space")
            .parse(present(space, `the home of ${residency.code}`));

        // wait until its forge serves
        const version = Deployment.#version(build, forgePackage.id);
        await this.#waitServed(
            residency,
            machine,
            home,
            FORGE_INSTALLATION,
            forgePackage.id,
            version,
        );
    }

    /** Sign a person up in a residency with a mailed code, as the CLI does, awaiting their home. */
    async #smokeSignUp(
        residency: PlatformResidency,
        custody: Custody,
        mail: PrintedMail,
    ): Promise<void> {
        // mail a code to a fresh address, and read it from the universe's printed mail
        const { environment } = this;
        const send = (request: Request) => environment.network.fetch(request);
        const issuer = new Issuer(this.issuer, send, environment, new Bookmark());
        const email = `e2e-${environment.random.bytes(4).toHex()}@example.com`;
        const printed = mail.lines.length;
        await issuer.sendCode(email);
        const code = await mail.code(printed, environment.clock);

        // redeem it on a new client, signing the person up in the residency
        const { userId } = await issuer.redeemCode(
            Identifier.create("client", environment),
            email,
            code,
            {
                name: "E2E",
                handle: `e2e-${environment.random.bytes(4).toHex()}`,
                residencyId: residency.code,
                timeZone: "UTC",
                locale: "en",
            },
        );

        // wait until their home serves every installation of its stack, as its readiness says
        await this.#waitReady(residency, Account.home(Account.personal(userId)));
    }

    /** Open the residency homes' forges, located as their machines, called as the operator. */
    forges(custody: Custody, build: PlatformBuild, operator: Operator): ForgeOpen {
        return async (handle) => {
            // find the handle's account and its residency's home at the universe, as a machine signed in
            const { environment } = this;
            const [first] = RESIDENCIES;
            const signedIn = await this.#actAs(present(first, "a residency"), custody);
            const accounts = createClient(
                accountPackage,
                {
                    url: PackageMount.url(this.universeOrigin, accountPackage.id),
                    fetch: signedIn.fetch(accountPackage.id),
                },
                environment,
            );
            const owner = await accounts.claim.owner(Claim.key(account, "handle", [handle]));
            const accountId = account.identifier(present(owner, `@${handle}`).objectId);
            const located = await accounts.residency.locate({ account: accountId });
            const home = schema.identifier("space").parse(located);

            // find the residency whose home it is, and wait until its machine serves the forge there
            const homes = await Promise.all(
                RESIDENCIES.map(async (each) => ({
                    residency: each,
                    space: (await accounts.residency.get({ id: each.code })).space,
                })),
            );
            const residency = present(
                homes.find((each) => each.space === home),
                `the residency of ${home}`,
            ).residency;
            const machine = await this.#actAs(residency, custody);
            const version = Deployment.#version(build, forgePackage.id);
            await this.#waitServed(
                residency,
                machine,
                home,
                FORGE_INSTALLATION,
                forgePackage.id,
                version,
            );

            // call the forge below its installation in the home, as the operator bound to the home
            const fetch = operator.fetch(registryPackage.id, { partition: home });
            const origin = InstallationMount.url(
                this.machineOrigin(residency),
                home,
                FORGE_INSTALLATION,
            );
            const url = `${origin}${PackageMount.path(registryPackage.id)}`;

            return {
                accountId,
                registry: new Registry(async () => ({ url, space: home, fetch }), environment),
            };
        };
    }

    /** Act as a residency's cloud machine, with the key its custody keeps, as its Worker does. */
    async #actAs(residency: PlatformResidency, custody: Custody): Promise<MachineActor> {
        // open the machine's key from its envelope
        const { environment } = this;
        const { id } = residency.machine;
        const keys = custody.machine(id);
        const envelope = new Envelope(keys.envelope, Envelope.machine(id), undefined, environment);
        const keychain = new MemoryKeychain();
        await keychain.save(id, await envelope.open(keys.key));
        const identity = new MachineKey(
            schema.identifier("machine").parse(id),
            keychain,
            environment,
        );
        const issuer = this.issuer;
        const send = (request: Request) => environment.network.fetch(request);

        // send as the machine toward a package, bound to a space when given
        const session = new Session(identity.at(issuer, send), {
            universe: this.universeOrigin,
            send,
            bookmark: new Bookmark(),
            environment,
        });

        return {
            fetch: (audience, partition) =>
                session.fetch(audience, partition === undefined ? {} : { partition }),
        };
    }

    /** Wait until a space's installation serves on a residency's machine, as its readiness says. */
    async #waitServed(
        residency: PlatformResidency,
        machine: MachineActor,
        space: Identifier<"space">,
        installation: string,
        packageId: PackageId,
        version: string,
    ): Promise<void> {
        // ask the installation's package for its readiness below its mount on the machine, speaking the release
        const mount = InstallationMount.url(this.machineOrigin(residency), space, installation);
        const request = new Request(`${mount}${PackageMount.path(packageId)}/readyz`, {
            redirect: "manual",
            headers: { [VERSION_HEADER]: version },
        });
        const fetch = machine.fetch(packageId, space);

        // ask again as each answer's Retry-After says, while the directory syncs and the space serves
        const { clock } = this.environment;
        const deadline = clock.now() + DIRECTORY_SYNC_MILLISECONDS + HOME_SERVE_MILLISECONDS;
        let response = await fetch(request.clone());
        let asked = response.headers.get("retry-after");
        while (response.status === 503 && asked !== null && clock.now() < deadline) {
            await response.body?.cancel();
            await clock.sleep(Number(asked) * 1000);
            response = await fetch(request.clone());
            asked = response.headers.get("retry-after");
        }

        // refuse an answer of a failure
        if (response.status >= 400) {
            const body = await response.text();
            throw new Error(
                `${installation} in ${space} on ${residency.code} answered ${response.status}: ${body}`,
            );
        }
    }

    /** Wait until a space on a residency's machine is ready, polling its readiness probe. */
    async #waitReady(residency: PlatformResidency, space: Identifier<"space">): Promise<void> {
        // ask the space's probe until it answers ready, or the deadline passes
        const { clock, network } = this.environment;
        const url = `${SpaceMount.url(this.machineOrigin(residency), space)}/readyz`;
        const deadline = clock.now() + DIRECTORY_SYNC_MILLISECONDS + HOME_SERVE_MILLISECONDS;
        for (;;) {
            // take a ready answer, refuse at the deadline with the last reason, else ask again
            const response = await network.fetch(new Request(url));
            const body = await response.text();
            if (response.ok) {
                return;
            } else if (clock.now() >= deadline) {
                throw new Error(
                    `${space} on ${residency.code} is not ready: ${response.status} ${body}`,
                );
            } else {
                await clock.sleep(READY_POLL_MILLISECONDS);
            }
        }
    }

    /** Read the version of a package a build releases. */
    static #version(build: PlatformBuild, packageId: PackageId): string {
        const reader = build.readers.find(
            (each) => PackageManifest.package(each.manifest).id === packageId,
        );

        return PackageManifest.package(present(reader, `the release of ${packageId}`).manifest)
            .version;
    }

    /** The URL of the universe's issuer. */
    get issuer(): string {
        return UniverseOrigin.issuer(this.universeOrigin);
    }

    /** Apply the global root, the universe's and the residencies', then release the Workers. */
    async deploy(print: (name: string, line: string) => void): Promise<void> {
        const { environment } = this;

        // apply every root in dependency order
        for (const root of Root.all(this.stage.name, environment)) {
            await root.operate("deploy");
        }

        // release the Workers onto the applied roots
        await this.release(print);
    }

    /** Migrate the universe's database, and deploy the universe and every cloud machine. */
    async release(print: (name: string, line: string) => void): Promise<void> {
        // read the operator, the roots' outputs and the custody record the operator sealed
        const { environment } = this;
        const operator = Root.required("DESTACK_OPERATOR", environment);
        const { outputs, outputsOf } = await this.#outputs();
        const custody = await Custody.read(
            Custody.file(this.stage.name),
            Deployment.#envelopes(outputs, outputsOf),
            environment,
        );

        // declare the universe over the record
        await using build = await this.#build();
        const declaration: UniverseDefinition = await this.universe(
            operator,
            custody,
            build.stacks(this.stage.tag),
            await this.homes(build),
        );

        // write the releases the Workers ship as their static assets
        await build.writeAssets(this.releases);
        print("release", `the Workers ship ${build.readers.length} releases`);

        // deploy every script in order, the universe first and each machine before its tail
        const scripts = this.#scripts(outputs, outputsOf);
        const modules = await Deployment.#modules(build, declaration);
        const providers = this.providers((id) =>
            Root.required(Deployment.providerSecret(id), environment),
        );
        for (const [script, configuration] of scripts) {
            const { target } = script;
            const access =
                target.kind === "universe" ? outputs.access : outputsOf(target.residency).access;
            const secrets = script.secrets(custody, access, providers);
            await script.deploy(configuration, secrets, modules);
        }

        // check every deployed origin answers, a tail answering none
        for (const [script] of scripts) {
            if (script.target.kind !== "tail") {
                await script.smoke();
            }
        }
    }

    /** Publish the stage's releases into the platform accounts' forge as the release workflow. */
    async publish(print: (name: string, line: string) => void): Promise<void> {
        // sign in as the workflow's trusted publisher, by the job's OIDC token for the token endpoint
        const { environment } = this;
        const send = (request: Request) => environment.network.fetch(request);
        const signer = {
            assert: (endpoint: string) => ActionsToken.request(endpoint, environment),
        };
        const session = new Session(new IssuerCredential(signer, this.issuer, send, environment), {
            universe: this.universeOrigin,
            send,
            bookmark: new Bookmark(),
            environment,
        });
        const accounts = session.accounts();

        // find each handle's account, and the platform account's home, which serves its forge
        const owner = async (handle: string) => {
            const claimed = await accounts.claim.owner(Claim.key(account, "handle", [handle]));
            if (claimed === null) {
                throw new Error(`no account claims @${handle}`);
            }

            return account.identifier(claimed.objectId);
        };
        const home = Account.home(await owner(PLATFORM.handle));

        // call the forge on the platform residency's cloud machine, as the trusted publisher bound to the home
        const residency = RESIDENCIES.find((each) => each.code === PLATFORM.residencyId);
        if (residency === undefined) {
            throw new Error(`no residency ${PLATFORM.residencyId} serves the platform`);
        }
        const origin = InstallationMount.url(
            this.machineOrigin(residency),
            home,
            FORGE_INSTALLATION,
        );
        const url = `${origin}${PackageMount.path(registryPackage.id)}`;
        const fetch = session.fetch(registryPackage.id, { partition: home });
        const registry = new Registry(async () => ({ url, space: home, fetch }), environment);

        // release each build under the stage's tag, in the account of its handle
        await using build = await this.#build();
        await Publication.release(
            build,
            this.stage.tag,
            async (handle) => ({ accountId: await owner(handle), registry }),
            async (reader) => {
                const { name, version } = PackageManifest.package(reader.manifest);
                print("publish", `${name}@${version}`);
            },
        );
    }

    /** Seal the keys the stage's custody record lacks under each root's KMS key, for committing. */
    async seal(print: (name: string, line: string) => void): Promise<void> {
        // seal as the operator, with the AWS key allowed to generate and encrypt under the roots' keys
        const { environment } = this;
        const credentials = {
            accessKeyId: Root.required("AWS_ACCESS_KEY_ID", environment),
            secretAccessKey: Root.required("AWS_SECRET_ACCESS_KEY", environment),
            ...sessionToken(environment.process.variables["AWS_SESSION_TOKEN"]),
        };

        // seal the keys the record lacks under the applied roots' envelopes
        const { outputs, outputsOf } = await this.#outputs();
        const file = Custody.file(this.stage.name);
        await Custody.seal(
            file,
            Deployment.#envelopes(outputs, outputsOf),
            credentials,
            environment,
        );
        print("custody", `sealed ${file}: commit it before the next deploy`);
    }

    /** Read the applied universe and residency roots' outputs. */
    async #outputs(): Promise<{
        readonly outputs: UniverseOutputs;
        readonly outputsOf: (residency: PlatformResidency) => ResidencyOutputs;
    }> {
        // read the universe root's outputs
        const { environment } = this;
        const universe = new Root([this.stage.name, "universe"], environment);
        const outputs = await universe.universe(await universe.initialize());

        // read each residency root's outputs
        const residencies = new Map<string, ResidencyOutputs>();
        for (const residency of RESIDENCIES) {
            const deployment = new Root([this.stage.name, residency.code], environment);
            const variables = await deployment.initialize();
            residencies.set(residency.code, await deployment.residency(variables));
        }
        const outputsOf = (residency: PlatformResidency) =>
            present(residencies.get(residency.code), `the outputs of ${residency.code}`);

        return { outputs, outputsOf };
    }

    /** List the KMS envelopes the custody record seals under: the universe's and each machine's. */
    static #envelopes(
        outputs: UniverseOutputs,
        outputsOf: (residency: PlatformResidency) => ResidencyOutputs,
    ): CustodyEnvelopes {
        return {
            universe: outputs.envelope,
            machines: new Map(
                RESIDENCIES.map((residency) => [
                    residency.machine.id,
                    outputsOf(residency).envelope,
                ]),
            ),
        };
    }

    /** Declare the platform's domains and its residency accounts' hostnames, none locally. */
    #edge(): Pick<PlatformDefinition, "domains" | "hostnames"> {
        // declare the space, machine, universe and site domains, each relay's host, and the edge's names at the platform's
        const edge = [`*.${DOMAINS.space}`, `*.${DOMAINS.machine}`, DOMAINS.universe, DOMAINS.site];
        const hostnames = RESIDENCIES.map((residency): [string, string[]] => [
            residency.code,
            [
                `relay-${residency.code}.${DOMAINS.machine}`,
                ...(residency.code === PLATFORM.residencyId ? edge : []),
            ],
        ]);

        return {
            domains: [DOMAINS.space, DOMAINS.machine, DOMAINS.universe, DOMAINS.site],
            hostnames: Object.fromEntries(hostnames),
        };
    }

    /** Name the variable a deploy reads a sign-in provider's client secret from. */
    static providerSecret(id: ProviderId): string {
        return PROVIDER_SECRETS[id];
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

    /** List the cloud machines of every residency. */
    #machineIds(): string[] {
        return RESIDENCIES.map((residency) => residency.machine.id);
    }

    /** Build the checkout's first-party releases into the stage's store. */
    async #build(local?: LocalBuild): Promise<PlatformBuild> {
        return await PlatformBuild.build(
            REPOSITORY,
            resolve(REPOSITORY, "target", this.stage.name, "releases"),
            this.environment,
            local,
        );
    }

    /** Write the universe root's outputs an end-to-end run binds, its files at an S3 endpoint. */
    #placeholderUniverse(endpoint: URL): UniverseOutputs {
        return {
            account: PLACEHOLDER_IDENTIFIER,
            hyperdrive: PLACEHOLDER_IDENTIFIER,
            files: { endpoint, bucket: this.#files, region: PLACEHOLDER_REGION },
            ...PLACEHOLDER_WORKER,
        };
    }

    /** The name of the stage's universe files bucket. */
    get #files(): string {
        return `destack-${this.stage.name}-files-universe`;
    }

    /** Write a residency root's outputs an end-to-end run binds. */
    #placeholderResidency(residency: PlatformResidency): ResidencyOutputs {
        const host = new URL(this.machineOrigin(residency)).host;

        return {
            account: PLACEHOLDER_IDENTIFIER,
            files: `destack-${this.stage.name}-files-${residency.code}`,
            packages: `destack-${this.stage.name}-packages-${residency.code}`,
            host,
            hyperdrive: PLACEHOLDER_IDENTIFIER,
            ...PLACEHOLDER_WORKER,
        };
    }

    /** Supply the bundles with the distribution, the universe and the in-process servers. */
    static async #modules(
        build: PlatformBuild,
        declaration: UniverseDefinition,
    ): Promise<SuppliedModules> {
        return {
            [DISTRIBUTION_MODULE]: { kind: "data", value: await build.distribution() },
            [UNIVERSE_MODULE]: { kind: "data", value: declaration },
            [SERVER_BUNDLE_MODULE]: { kind: "imports", specifiers: build.serverBundle() },
        };
    }

    /** Configure the scripts in deploy order: the universe, then each tail and machine. */
    #scripts(
        universe: UniverseOutputs,
        residency: (residency: PlatformResidency) => ResidencyOutputs,
    ): (readonly [WorkerScript, WranglerConfiguration])[] {
        // configure the universe binding its declaration
        const script = new WorkerScript(this, { kind: "universe" });
        const scripts = [[script, script.universe(universe)] as const];

        // configure each residency's cloud machine before the tail binding its objects
        for (const served of RESIDENCIES) {
            const tail = new WorkerScript(this, { kind: "tail", residency: served });
            const machine = new WorkerScript(this, { kind: "machine", residency: served });
            scripts.push(
                [machine, machine.machine(residency(served), served)],
                [tail, tail.tail(served)],
            );
        }

        return scripts;
    }
}

/** Read an AWS session token of temporary credentials, none when unset. */
function sessionToken(token: string | undefined): { readonly sessionToken?: string } {
    return token === undefined || token === "" ? {} : { sessionToken: token };
}
