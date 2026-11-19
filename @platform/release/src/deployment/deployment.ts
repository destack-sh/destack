import { TelemetryScope } from "@destack/telemetry";
import { DeclarationReference } from "@destack/package/declare";
import { UNIT_SKUS } from "@platform/pricing";
import type { ResidencyCode } from "@destack/account/object";
import { DOMAINS, UniverseOrigin } from "@destack/account/address";
import { ProviderId } from "@destack/account/authentication";
import type { ProviderClients } from "@destack/account/better-auth";
import type { Jurisdiction } from "@destack/db/cloudflare";
import { Universe } from "@destack/update/release";
import { Identifier, type JsonValue, present, schema } from "@destack/schema";
import type { UniverseDefinition } from "@destack/control";
import type { WorkspaceUnitOptions } from "@destack/build";
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
import { Claim } from "@destack/object";
import registryPackage from "@destack/registry/package";
import { ActionsToken } from "../github/oidc.ts";
import { Operator } from "../local/operator.ts";
import { PrintedMail } from "../local/mail.ts";
import { Bookmark } from "@destack/object/client";
import { VERSION_HEADER } from "@destack/object";
import type { Fetch } from "@destack/object";
import * as relay from "@destack/relay/declare";
import { join, resolve } from "node:path";
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
import { CHECKOUT } from "../distribution/distribution.ts";
import { type ForgeOpen, PlatformBuild } from "./build.ts";
import { GITHUB_APP, PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { Custody, type CustodyEnvelopes } from "./custody.ts";
import { DISTRIBUTION_MODULE, PackageId } from "@destack/package";
import { PackageManifest } from "@destack/package/manifest";
import { LATEST_TAG } from "@destack/registry/object";
import { SERVER_BUNDLE_MODULE } from "@destack/machine/cloudflare";
import { InstallationMount, PackageMount } from "@destack/object";
import {
    type ClientSocket,
    type Environment,
    type HostCapability,
    type LogFormat,
    type ServerSocket,
    untilAborted,
    WebSocketClosure,
    WebSocketMessage,
    WebSocketUpgrade,
} from "@destack/environment";
import { BunNetwork } from "@destack/environment/bun";

/** The status the front answers an upgrade with when the Worker's socket does not open. */
const BAD_GATEWAY = 502;

/** How long a started machine's directory replica may take to sync: a minute. */
const DIRECTORY_SYNC_MILLISECONDS = 60_000;

/** The directory below a stage's Worker storage where its Workers find each other. */
const WORKER_REGISTRY = "registry";

/** How long a deployed residency home may take to serve: seconds from cached builds. */
const HOME_SERVE_MILLISECONDS = 30_000;

/** The identifier a run under workerd binds where a deployment's state has a real one. */
const PLACEHOLDER_IDENTIFIER = "0".repeat(32);

/** The offset of a script's inspector port from its own port, clear of the loopback ports. */
const INSPECTOR_OFFSET = 5000;

/** The configuration keys a run under workerd leaves out: routes, tails and artifacts. */
const DEPLOYED_KEYS = new Set(["routes", "tail_consumers", "artifacts"]);

/** The directory below a universe's target a run under workerd keeps its files in. */
const WORKERD_DIRECTORY = "workerd";

/** A residency's cloud machine a check acts as: its sends toward packages, bound to a space. */
interface MachineActor {
    /** Send as the machine toward a package, bound to a space when given. */
    readonly fetch: (audience: PackageId, partition?: Identifier<"space">) => Fetch;
}

/** What a universe's local Workers share as they start: modules, keys, storage, mail, processes. */
interface LocalWorkers {
    /** The modules the Workers bundle. */
    readonly modules: SuppliedModules;
    /** The keys the Workers' secrets come from. */
    readonly custody: Custody;
    /** The placeholder providers the Workers call. */
    readonly placeholders: ProviderClients;
    /** The directory each Worker keeps its storage below. */
    readonly storage: string;
    /** The local Hyperdrive connection to the universe's database. */
    readonly hyperdrive: URL;
    /** The mail the universe prints, read for sign-in codes. */
    readonly mail: PrintedMail;
    /** The Worker processes, stopped with the universe. */
    readonly children: AsyncDisposableStack;
    /** Print a Worker's line under its service. */
    readonly print: (name: string, line: string) => void;
}

/** The password local Hyperdrive requires, which the trusting local server ignores. */
const HYPERDRIVE_PASSWORD = "local";

/** The prefix of wrangler's own information lines, such as its access log. */
const WRANGLER_INFO_PREFIX = "[wrangler:info]";

/** The region a run under workerd's files bucket signs for. */
const PLACEHOLDER_REGION = "local";

/** The access key a local run binds in place of a provisioned one. */
const PLACEHOLDER_KEY = { accessKeyId: "local", secretAccessKey: "local" };

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

/** What a local run's roots provision for a Worker, whose local custody envelopes nothing. */
const PLACEHOLDER_WORKER: WorkerOutputs = { envelope: { kind: "local" }, access: PLACEHOLDER_KEY };

/** The loopback interface the local universe's processes listen on. */
const LOOPBACK = "127.0.0.1";

/** The domain the cloud machines answer below, each one label as `<machine>--platform`. */
const MACHINE_DOMAIN = "destack.computer";

/** The provider running the cloud machines. */
const PROVIDER = "cloudflare";

/** The local universe's loopback port, which the universe's other ports follow. */
const UNIVERSE_PORT = Number(new URL(Universe.of("local").origin).port);

/** The loopback port each service of the local universe listens on. */
export interface LoopbackLayout {
    /** The universe's port, its issuer's. */
    readonly universe: number;
    /** This computer's own machine's port, which the local universe's people use. */
    readonly machine: number;
    /** Each residency's cloud machine's port, by residency code. */
    readonly machines: Readonly<Record<ResidencyCode, number>>;
    /** The universe Worker's own port, behind the universe port's edge under workerd. */
    readonly universeWorker: number;
}

/** The loopback layouts of local universes. */
export const LoopbackLayout = {
    /** Lay a local universe out from its universe's port, the other services following it. */
    at(universe: number): LoopbackLayout {
        return {
            universe,
            machine: universe + 1,
            machines: Object.fromEntries(
                RESIDENCIES.map((residency, index) => [residency.code, universe + 2 + index]),
            ),
            universeWorker: universe + 2 + RESIDENCIES.length,
        };
    },
};

/** The loopback layout of the default local universe. */
export const LOOPBACK_LAYOUT = LoopbackLayout.at(UNIVERSE_PORT);

/** The platform deployed at a universe: its universe, residencies and Workers. */
export class Deployment {
    /** The universe deployed, at its origin. */
    readonly universe: Universe;
    /** The loopback port each service of the local universe listens on. */
    readonly layout: LoopbackLayout;
    /** The host the deployment's tools run on. */
    readonly environment: Environment<HostCapability>;
    /** The deployment's telemetry, whose records print as status lines. */
    readonly scope: TelemetryScope;

    /** Deploy the platform at a universe from a host. */
    private constructor(
        universe: Universe,
        layout: LoopbackLayout,
        environment: Environment<HostCapability>,
    ) {
        // keep the universe, a local one at its layout's port, its layout and its host
        this.universe =
            universe.name === "local"
                ? new Universe(universe.name, `http://${LOOPBACK}:${layout.universe}`)
                : universe;
        this.layout = layout;
        this.environment = environment;
        this.scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    }

    /** Read the deployment of a universe by its name, refusing any other. */
    static of(name: string | undefined, environment: Environment<HostCapability>): Deployment {
        return new Deployment(Universe.of(name), LOOPBACK_LAYOUT, environment);
    }

    /** The format the universe's Workers print in: lines locally, JSON for Workers Logs. */
    get logFormat(): LogFormat {
        return this.universe.name === "local" ? "line" : "json";
    }

    /** Read the local universe's deployment listening on a loopback layout. */
    static loopback(layout: LoopbackLayout, environment: Environment<HostCapability>): Deployment {
        return new Deployment(Universe.of("local"), layout, environment);
    }

    /** Read a deployed universe's deployment by its name, refusing the local one and any other. */
    static deployed(
        name: string | undefined,
        environment: Environment<HostCapability>,
    ): Deployment {
        const deployment = Deployment.of(name, environment);
        if (deployment.universe.name === "local") {
            throw new Error("the local universe runs under just local and deploys nothing");
        }

        return deployment;
    }

    /** The directory of the asset layers, one directory per layer under its digest. */
    get assets(): string {
        return resolve(CHECKOUT, "target", this.universe.name, "cloud");
    }

    /** The origin the universe's Worker answers at: its own port behind the edge locally. */
    get universeWorkerOrigin(): string {
        return this.universe.name === "local"
            ? `http://${LOOPBACK}:${this.layout.universeWorker}`
            : this.universe.origin;
    }

    /** Address a request at the universe's domain, as the edge does. */
    edge(request: Request): Request {
        // keep the path and query below the universe's domain
        const requested = new URL(request.url);
        const routed = new URL(
            `${requested.pathname}${requested.search}`,
            `https://${DOMAINS.universe}`,
        );

        // present the browser's own origin as the universe's, as a reverse proxy changing the origin does
        const headers = new Headers(request.headers);
        if (headers.get("origin") === requested.origin) {
            headers.set("origin", routed.origin);
        }
        const referer = headers.get("referer");
        if (referer?.startsWith(`${requested.origin}/`) === true) {
            headers.set("referer", `${routed.origin}${referer.slice(requested.origin.length)}`);
        }

        return new Request(new Request(routed, request), { headers });
    }

    /** The origin the cloud machine serving a residency answers at. */
    machineOrigin(residency: PlatformResidency): string {
        // answer on its loopback port locally
        if (this.universe.name === "local") {
            const port = this.layout.machines[residency.code];

            return `http://${LOOPBACK}:${present(port, `the port of ${residency.code}`)}`;
        }

        return `https://${residency.machine.name}--platform.${MACHINE_DOMAIN}`;
    }

    /** The origin machines open their tunnels at to a residency's relay, its machine's locally. */
    relayOrigin(residency: PlatformResidency): string {
        return this.universe.name === "local"
            ? this.machineOrigin(residency)
            : `https://relay-${residency.code}.${DOMAINS.machine}`;
    }

    /** Select each residency home's stack with its relay's settings and production's routes. */
    async homes(build: Pick<PlatformBuild, "released">): Promise<Record<string, StackRelease>> {
        const homes: Record<string, StackRelease> = {};
        for (const residency of RESIDENCIES) {
            homes[residency.code] = build.released(residency.stack, {
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
        return this.universe.name === "local" ? "none" : residency.jurisdiction;
    }

    /** Where the universe's Workers send mail: through SES in production, else to their logs. */
    get mail(): MailSender {
        return this.universe.name === "production"
            ? { kind: "ses", region: MAIL_REGION, from: MAIL_FROM }
            : { kind: "print" };
    }

    /** The setting values the residency homes' platform stack places, by setting key. */
    get settings(): Readonly<Record<string, JsonValue>> {
        return {
            [DeclarationReference.key(tag.reference)]: this.universe.tag,
            [DeclarationReference.key(github.reference)]:
                this.universe.name === "production"
                    ? { kind: "app", appId: GITHUB_APP }
                    : { kind: "none" },
            [DeclarationReference.key(provider.reference)]:
                this.universe.name === "production" ? "stripe" : "none",
        };
    }

    /** Declare the universe's definition. */
    async definition(
        operator: string,
        custody: Custody,
        stacks: PlatformDefinition["stacks"],
        homes: PlatformDefinition["homes"],
    ): Promise<UniverseDefinition> {
        // declare each residency's cloud machine under the key its custody keeps, rating its spaces by the platform's SKUs
        const rates = Object.fromEntries(
            UNIT_SKUS.map((sku) => [sku.meterKey, DeclarationReference.key(sku.reference)]),
        );
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
                rates,
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

    /** Serve the local universe and cloud machines under workerd until a hold ends or a stop. */
    async serve(
        server: string,
        database: string,
        custody: Custody,
        storage: string,
        print: (name: string, line: string) => void,
        hold: (build: PlatformBuild) => Promise<void>,
        signal: AbortSignal,
    ): Promise<void> {
        // address the state's database, which keeps the universe between runs
        const { environment } = this;
        const url = `${server}/${database}`;

        // connect local Hyperdrive with the password it requires
        const hyperdrive = new URL(url);
        hyperdrive.password ||= HYPERDRIVE_PASSWORD;

        // build the checkout
        await using build = await this.#build({ isPrerelease: true });

        // declare the universe over the state's keys with the stacks built from the checkout
        const declaration = await this.definition(
            Operator.LOCAL,
            custody,
            build.stacks(this.universe.tag),
            await this.homes(build),
        );

        // serve the universe's files from a local bucket over S3
        await using files = await LocalBucket.open(
            resolve(CHECKOUT, "target", this.universe.name, WORKERD_DIRECTORY, "files"),
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

        // write the releases the Workers ship as their static assets
        const releases = await build.writeAssets(this.assets);

        // configure the scripts over placeholder outputs
        const scripts = this.#scripts(
            this.#placeholderUniverse(listener.url),
            (residency) => this.#placeholderResidency(residency),
            releases,
        ).filter(([script]) => script.target.kind !== "tail");
        const modules = await Deployment.#modules(build, declaration);

        // serve the universe's port as Cloudflare's routes do: its own paths at the universe's Worker, others at the platform residency's machine
        const platform = present(
            RESIDENCIES.find((residency) => residency.code === PLATFORM.residencyId),
            "the platform residency",
        );
        const front = environment.server.http<UpstreamSocket>({
            hostname: LOOPBACK,
            port: this.layout.universe,
            fetch: async (request) => {
                // send the universe's paths to its Worker, others at its domain to the platform machine, passing bodies and redirects through
                const isUniverse = UniverseOrigin.path(new URL(request.url).pathname) !== undefined;
                const edged = isUniverse ? request : this.edge(request);
                const routed = new URL(edged.url);
                const target = isUniverse
                    ? this.universeWorkerOrigin
                    : this.machineOrigin(platform);
                const headers = new Headers(edged.headers);
                headers.set("host", routed.host);

                // forward a WebSocket upgrade on a socket of its own to the same target
                if (WebSocketUpgrade.of(edged) !== undefined) {
                    const upstream = await UpstreamSocket.open(
                        new Request(edged.url, { headers }),
                        `${target.replace(/^http/u, "ws")}${routed.pathname}${routed.search}`,
                    );

                    return upstream === undefined
                        ? new Response(null, { status: BAD_GATEWAY })
                        : {
                              attachment: upstream,
                              ...(upstream.protocol === ""
                                  ? {}
                                  : { headers: { "sec-websocket-protocol": upstream.protocol } }),
                          };
                }

                return environment.network.fetch(
                    new Request(new Request(`${target}${routed.pathname}${routed.search}`, edged), {
                        headers,
                        redirect: "manual",
                    }),
                    { decompress: false },
                );
            },
            // relay each forwarded socket's messages and close to its worker's end
            websocket: {
                open: (socket) => socket.attachment.attach(socket),
                message: (socket, message) => socket.attachment.send(message),
                close: (socket, code, reason) => socket.attachment.close(code, reason),
            },
        });
        serving.defer(() => front.stop(true));

        // serve each script on its own ports with placeholder secrets and its root key, and check it answers, keeping the universe's lines
        await using children = new AsyncDisposableStack();
        const mail = new PrintedMail();
        const placeholders = this.providers(() => PLACEHOLDER_KEY.secretAccessKey);
        const workers: LocalWorkers = {
            modules,
            custody,
            placeholders,
            storage,
            hyperdrive,
            mail,
            children,
            print,
        };

        // start the Workers, ending the start at the signal, which stops the started Workers on the way out
        await untilAborted(this.#start(scripts, workers, custody, build, mail), signal);
        if (signal.aborted) {
            return;
        }

        // serve the build until the hold ends
        await hold(build);
    }

    /** Start the Workers, wait for the residency homes and publish the releases. */
    async #start(
        scripts: readonly (readonly [WorkerScript, WranglerConfiguration])[],
        workers: LocalWorkers,
        custody: Custody,
        build: PlatformBuild,
        mail: PrintedMail,
    ): Promise<void> {
        // start the universe first, which the machines bind, then the machines at once
        const [universeScript, ...machineScripts] = scripts;
        const [universeWorker, universeConfiguration] = present(
            universeScript,
            "the universe's script",
        );
        await this.#startWorker(universeWorker, universeConfiguration, workers);
        await Promise.all(
            machineScripts.map(([script, configuration]) =>
                this.#startWorker(script, configuration, workers),
            ),
        );

        // wait until each residency's home serves its forge on its cloud machine, the residencies at once
        await Promise.all(
            RESIDENCIES.map((residency) =>
                this.scope.span(
                    "platform.residency.serve",
                    { "platform.residency.code": residency.code },
                    () => this.#waitForgeReady(residency, custody, build),
                    { level: "info" },
                ),
            ),
        );

        // publish the platform's releases as the local operator
        await this.publishLocal(build, custody, mail);
    }

    /** Start a script's local workerd and wait until it answers, recording its start. */
    async #startWorker(
        script: WorkerScript,
        configuration: WranglerConfiguration,
        workers: LocalWorkers,
    ): Promise<void> {
        // start it in a span, which records the start once the Worker answers
        const { environment } = this;
        const { modules, custody, placeholders, storage, hyperdrive, mail, children, print } =
            workers;
        await this.scope.span(
            "platform.worker.start",
            { "platform.worker.name": script.service, "url.full": script.origin },
            async () => {
                // write the configuration without what only a deployment binds
                const local = Object.fromEntries(
                    Object.entries(configuration).filter(([key]) => !DEPLOYED_KEYS.has(key)),
                );
                const path = await script.write(local, modules);
                const secrets = script.secrets(custody, PLACEHOLDER_KEY, placeholders);
                const variables = Object.entries(secrets).map(
                    ([name, value]) => `${name}=${value}\n`,
                );
                await environment.files.writeTextFile(
                    resolve(path, "..", ".dev.vars"),
                    variables.join(""),
                    { mode: 0o600 },
                );

                // start local workerd, waiting until it serves or exits
                const port = Number(new URL(script.origin).port);
                const child = children.use(
                    LocalProcess.start(
                        script.service,
                        {
                            command: [
                                resolve(ROOT, "node_modules/.bin/wrangler"),
                                "dev",
                                "--local",
                                // keep each Worker's storage in the state between runs, apart from its rebuilt bundle
                                "--persist-to",
                                join(storage, script.service),
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
                                ...Root.inherited(
                                    environment.process.variables,
                                    WRANGLER_VARIABLES,
                                ),
                                CLOUDFLARE_HYPERDRIVE_LOCAL_CONNECTION_STRING_DATABASE:
                                    hyperdrive.href,
                                WRANGLER_SEND_METRICS: "false",
                                // register the stage's Workers with each other alone, apart from any other stage on this computer
                                WRANGLER_REGISTRY_PATH: join(storage, WORKER_REGISTRY),
                            },
                            print: (name, line) => {
                                // leave out wrangler's own information lines, its access log among them, as spans record each request
                                if (!line.startsWith(WRANGLER_INFO_PREFIX)) {
                                    print(name, line);
                                }
                                if (script.target.kind === "universe") {
                                    mail.push(line);
                                }
                            },
                        },
                        environment,
                    ),
                );
                const answered = await Promise.race([
                    script.waitReady(),
                    child.exited.then(() => undefined),
                ]);
                if (answered === undefined) {
                    throw new Error(`'${script.name}' exited before it served`);
                }
            },
            { level: "info" },
        );
    }

    /** Wait until a residency home's forge serves on the residency's cloud machine. */
    async #waitForgeReady(
        residency: PlatformResidency,
        custody: Custody,
        build: PlatformBuild,
    ): Promise<void> {
        // read the residency's home at the universe as the machine, which may locate it
        const machine = await this.#actAs(residency, custody);
        const accounts = createClient(
            accountPackage,
            {
                url: PackageMount.url(this.universe.origin, accountPackage.id),
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
                    url: PackageMount.url(this.universe.origin, accountPackage.id),
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
            const fetch = this.#waiting(operator.fetch(registryPackage.id, { partition: home }));
            const origin = InstallationMount.url(
                this.machineOrigin(residency),
                home,
                FORGE_INSTALLATION,
            );
            const url = `${origin}${PackageMount.path(registryPackage.id)}`;

            return { accountId, home, url, fetch };
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
            universe: this.universe.origin,
            send,
            bookmark: new Bookmark(),
            environment,
        });

        return {
            fetch: (audience, partition) =>
                session.fetch(audience, partition === undefined ? {} : { partition }),
        };
    }

    /** Send as a fetch does, asking again as a 503's Retry-After says, until a deadline. */
    #waiting(
        fetch: (request: Request) => Promise<Response>,
    ): (request: Request) => Promise<Response> {
        const { clock } = this.environment;

        return async (request) => {
            // ask again as each answer's Retry-After says, until the deadline
            const deadline = clock.now() + DIRECTORY_SYNC_MILLISECONDS + HOME_SERVE_MILLISECONDS;
            let response = await fetch(request.clone());
            let asked = response.headers.get("retry-after");
            while (response.status === 503 && asked !== null && clock.now() < deadline) {
                await response.body?.cancel();
                await clock.sleep(Number(asked) * 1000);
                response = await fetch(request.clone());
                asked = response.headers.get("retry-after");
            }

            return response;
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
        const response = await this.#waiting(machine.fetch(packageId, space))(request);

        // refuse an answer of a failure
        if (response.status >= 400) {
            const body = await response.text();
            throw new Error(
                `'${installation}' in '${space}' on '${residency.code}' answered ${response.status}: '${body}'`,
            );
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
        return UniverseOrigin.issuer(this.universe.origin);
    }

    /** Apply the global root, the universe's and the residencies', then release the Workers. */
    async deploy(): Promise<void> {
        const { environment } = this;

        // apply every root in dependency order
        for (const root of Root.all(this.universe.name, environment)) {
            await root.operate("deploy");
        }

        // release the Workers onto the applied roots
        await this.release();
    }

    /** Publish a build's releases as the local operator under the universe's tag and latest. */
    async publishLocal(build: PlatformBuild, custody: Custody, mail: PrintedMail): Promise<void> {
        // sign the operator in, and publish under the universe's tag as the platform's owner, pointing latest, which installs follow
        const send = (request: Request) => this.environment.network.fetch(request);
        const issuer = new Issuer(this.issuer, send, this.environment, new Bookmark());
        const operator = await Operator.signIn(issuer, mail, this.environment);
        await build.release(this.universe.tag, this.forges(custody, build, operator), {
            tags: [LATEST_TAG],
        });
    }

    /** Migrate the universe's database, and deploy the universe and every cloud machine. */
    async release(): Promise<void> {
        // read the operator, the roots' outputs and the custody record the operator sealed
        const { environment } = this;
        const operator = Root.required("DESTACK_OPERATOR", environment);
        const { outputs, outputsOf } = await this.#outputs();
        const custody = await Custody.read(
            Custody.file(this.universe.name),
            Deployment.#envelopes(outputs, outputsOf),
            environment,
        );

        // declare the universe over the record
        await using build = await this.#build();
        const declaration: UniverseDefinition = await this.definition(
            operator,
            custody,
            build.stacks(this.universe.tag),
            await this.homes(build),
        );

        // write the releases the Workers ship as their static assets
        const releases = await build.writeAssets(this.assets);
        this.scope.log.info("platform.release.shipped", {
            "platform.release.count": build.readers.length,
        });

        // deploy every script in order, the universe first and each machine before its tail
        const scripts = this.#scripts(outputs, outputsOf, releases);
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

        // check every deployed origin answers once it started, a tail answering none
        for (const [script] of scripts) {
            if (script.target.kind !== "tail") {
                const started = await script.waitReady();
                this.scope.log.info("platform.deployment.answered", {
                    "platform.script.name": script.name,
                    "platform.deployment.duration_ms": started,
                });
            }
        }
    }

    /** Publish the universe's releases into the platform's forge as the release workflow. */
    async publish(): Promise<void> {
        // sign in as the workflow's trusted publisher, by the job's OIDC token for the token endpoint
        const { environment } = this;
        const send = (request: Request) => environment.network.fetch(request);
        const signer = {
            assert: (endpoint: string) => ActionsToken.request(endpoint, environment),
        };
        const session = new Session(new IssuerCredential(signer, this.issuer, send, environment), {
            universe: this.universe.origin,
            send,
            bookmark: new Bookmark(),
            environment,
        });
        const accounts = session.accounts();

        // find each handle's account, and the platform account's home, which serves its forge
        const owner = async (handle: string) => {
            const claimed = await accounts.claim.owner(Claim.key(account, "handle", [handle]));
            if (claimed === null) {
                throw new Error(`no account claims '@${handle}'`);
            }

            return account.identifier(claimed.objectId);
        };
        const home = Account.home(await owner(PLATFORM.handle));

        // call the forge on the platform residency's cloud machine, as the trusted publisher bound to the home
        const residency = RESIDENCIES.find((each) => each.code === PLATFORM.residencyId);
        if (residency === undefined) {
            throw new Error(`no residency '${PLATFORM.residencyId}' serves the platform`);
        }
        const origin = InstallationMount.url(
            this.machineOrigin(residency),
            home,
            FORGE_INSTALLATION,
        );
        const url = `${origin}${PackageMount.path(registryPackage.id)}`;
        const fetch = session.fetch(registryPackage.id, { partition: home });

        // release each build under the universe's tag in the account of its handle, pointing latest, which installs follow
        await using build = await this.#build();
        await build.release(
            this.universe.tag,
            async (handle) => ({ accountId: await owner(handle), home, url, fetch }),
            { tags: [LATEST_TAG] },
        );
    }

    /** Seal the keys the universe's custody record lacks under each root's KMS key. */
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
        const file = Custody.file(this.universe.name);
        await Custody.seal(
            file,
            Deployment.#envelopes(outputs, outputsOf),
            credentials,
            environment,
        );
        this.scope.log.info("platform.custody.sealed", {
            "file.path": file,
        });
        print("custody", `commit ${file} before the next deploy`);
    }

    /** Read the applied universe and residency roots' outputs. */
    async #outputs(): Promise<{
        readonly outputs: UniverseOutputs;
        readonly outputsOf: (residency: PlatformResidency) => ResidencyOutputs;
    }> {
        // read the universe root's outputs
        const { environment } = this;
        const universe = new Root([this.universe.name, "universe"], environment);
        const outputs = await universe.universe(await universe.initialize());

        // read each residency root's outputs
        const residencies = new Map<string, ResidencyOutputs>();
        for (const residency of RESIDENCIES) {
            const deployment = new Root([this.universe.name, residency.code], environment);
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

    /** Build the checkout's first-party releases into the universe's store. */
    async #build(options: Pick<WorkspaceUnitOptions, "isPrerelease"> = {}): Promise<PlatformBuild> {
        return await PlatformBuild.build(CHECKOUT, this.environment, options);
    }

    /** Write the universe root's outputs a run under workerd binds, its files at an S3 endpoint. */
    #placeholderUniverse(endpoint: URL): UniverseOutputs {
        return {
            account: PLACEHOLDER_IDENTIFIER,
            hyperdrive: PLACEHOLDER_IDENTIFIER,
            files: { endpoint, bucket: this.#files, region: PLACEHOLDER_REGION },
            ...PLACEHOLDER_WORKER,
        };
    }

    /** The name of the universe's universe files bucket. */
    get #files(): string {
        return `destack-${this.universe.name}-files-universe`;
    }

    /** Write a residency root's outputs a run under workerd binds. */
    #placeholderResidency(residency: PlatformResidency): ResidencyOutputs {
        const host = new URL(this.machineOrigin(residency)).host;

        return {
            account: PLACEHOLDER_IDENTIFIER,
            bucket: `destack-${this.universe.name}-${residency.code}`,
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
            [DISTRIBUTION_MODULE]: { kind: "data", value: build.distribution("workerd") },
            [UNIVERSE_MODULE]: { kind: "data", value: declaration },
            [SERVER_BUNDLE_MODULE]: { kind: "imports", specifiers: build.serverBundle() },
        };
    }

    /** Configure the scripts in deploy order: the universe, then each tail and machine. */
    #scripts(
        universe: UniverseOutputs,
        residency: (residency: PlatformResidency) => ResidencyOutputs,
        releases: string,
    ): (readonly [WorkerScript, WranglerConfiguration])[] {
        // configure the universe binding its declaration
        const script = new WorkerScript(this, { kind: "universe" });
        const scripts = [[script, script.universe(universe, releases)] as const];

        // configure each residency's cloud machine before the tail binding its objects
        for (const served of RESIDENCIES) {
            const tail = new WorkerScript(this, { kind: "tail", residency: served });
            const machine = new WorkerScript(this, { kind: "machine", residency: served });
            scripts.push(
                [machine, machine.machine(residency(served), served, releases)],
                [tail, tail.tail(served)],
            );
        }

        return scripts;
    }
}

/** A client's WebSocket the local front forwards to a Worker: the Worker's end, opened first. */
class UpstreamSocket {
    /** The Worker's end. */
    readonly #upstream: ClientSocket;
    /** The Worker's messages that arrived before the client's end opened. */
    readonly #queued: (string | Uint8Array<ArrayBuffer>)[] = [];
    /** The client's end, once it opened. */
    #client: ServerSocket<UpstreamSocket> | undefined;
    /** The Worker's close before the client's end opened. */
    #closed: { readonly code: number; readonly reason: string } | undefined;

    /** Relay a Worker's socket to a client's end once it opens. */
    private constructor(upstream: ClientSocket) {
        this.#upstream = upstream;
        upstream.addEventListener("message", (event) => {
            // hand the client the message, keeping it until the client's end opened
            const message = WebSocketMessage.of(event.data);
            if (this.#client === undefined) {
                this.#queued.push(message);
            } else {
                this.#relay(this.#client, message);
            }
        });
        upstream.addEventListener("close", (event) => {
            // close the client's end, or close it as it opens
            if (this.#client === undefined) {
                this.#closed = { code: event.code, reason: event.reason };
            } else {
                this.#client.close(WebSocketClosure.sendable(event.code), event.reason);
            }
        });
    }

    /** The subprotocol the Worker chose. */
    get protocol(): string {
        return this.#upstream.protocol;
    }

    /** Open a Worker's socket for a forwarded upgrade, absent when it closes before it opens. */
    static async open(upgrade: Request, url: string): Promise<UpstreamSocket | undefined> {
        // open it with the upgrade's headers, and answer once it opens
        const upstream = BunNetwork.forward(upgrade, url);
        const isOpen = await new Promise<boolean>((settle) => {
            upstream.addEventListener("open", () => settle(true), { once: true });
            upstream.addEventListener("close", () => settle(false), { once: true });
        });

        return isOpen ? new UpstreamSocket(upstream) : undefined;
    }

    /** Take the client's end as it opens, handing it the Worker's earlier messages and close. */
    attach(client: ServerSocket<UpstreamSocket>): void {
        // send what arrived meanwhile, then close as the Worker did
        this.#client = client;
        for (const message of this.#queued.splice(0)) {
            this.#relay(client, message);
        }
        if (this.#closed !== undefined) {
            client.close(WebSocketClosure.sendable(this.#closed.code), this.#closed.reason);
        }
    }

    /** Send the Worker's message to the client, closing the Worker's end on a drop. */
    #relay(client: ServerSocket<UpstreamSocket>, message: string | Uint8Array<ArrayBuffer>): void {
        void client.send(message).catch(() => {
            this.#upstream.close(
                WebSocketClosure.goingAway,
                "the client's socket dropped a message",
            );
        });
    }

    /** Send the client's message to the Worker. */
    send(message: string | Uint8Array): void {
        this.#upstream.send(typeof message === "string" ? message : new Uint8Array(message));
    }

    /** Close the Worker's end as the client closed. */
    close(code: number, reason: string): void {
        this.#upstream.close(WebSocketClosure.sendable(code), reason);
    }
}

/** Read an AWS session token of temporary credentials, none when unset. */
function sessionToken(token: string | undefined): { readonly sessionToken?: string } {
    return token === undefined || token === "" ? {} : { sessionToken: token };
}
