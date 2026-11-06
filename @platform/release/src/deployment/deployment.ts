import { DeclarationReference } from "@destack/package/declare";
import type { ResidencyCode } from "@destack/account/object";
import { DOMAINS } from "@destack/account/address";
import type { Jurisdiction } from "@destack/db/cloudflare";
import { Stage } from "@destack/update/release";
import { type JsonValue, present } from "@destack/schema";
import type { UniverseDefinition } from "@destack/universe";
import type { PlatformDefinition } from "@destack/account/server";
import type { StackRelease } from "@destack/account/object";
import { provider, tag } from "@destack/finance/declare";
import { github } from "@destack/forge/stack";
import * as relay from "@destack/relay/declare";
import { UNIVERSE_SCHEMA, Universe } from "@destack/universe";
import { resolve } from "node:path";
import { connectPostgres, PostgresPool } from "@destack/db/postgres";
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
    type SharedOutputs,
    type UniverseOutputs,
    WRANGLER_VARIABLES,
} from "./root.ts";
import { LocalProcess } from "../local/process.ts";
import { PlatformBuild } from "./build.ts";
import { GITHUB_APP, PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { Custody, type CustodyEnvelopes } from "./custody.ts";
import { DISTRIBUTION_MODULE } from "@destack/package";
import type { Environment, HostCapability } from "@destack/environment";

/** The identifier a rehearsal binds where a deployment's state has a real one. */
const REHEARSAL_IDENTIFIER = "0".repeat(32);

/** The operator a rehearsal declares, where a release reads the real one from its environment. */
const REHEARSAL_OPERATOR = "operator@destack.test";

/** The offset of a script's inspector port from its own port, clear of the loopback ports. */
const INSPECTOR_OFFSET = 5000;

/** The configuration keys an end-to-end run leaves out: routes, tails and artifacts. */
const DEPLOYED_KEYS = new Set(["routes", "tail_consumers", "artifacts"]);

/** The password local Hyperdrive requires, which the trusting local server ignores. */
const HYPERDRIVE_PASSWORD = "local";

/** The line wrangler prints once local workerd serves a script. */
const READY_LINE = "Ready on";

/** The database a rehearsal's universe binds, which it never reaches. */
const REHEARSAL_DATABASE = "postgresql://rehearsal.invalid/postgres";

/** The repository's root, two levels above the release package. */
export const REPOSITORY = resolve(ROOT, "../..");

/** The access key a rehearsal and a local run bind in place of a provisioned one. */
const PLACEHOLDER_KEY = { accessKeyId: "local", secretAccessKey: "local" };

/** The shared root's outputs a rehearsal and a local run bind. */
const PLACEHOLDER_SHARED: SharedOutputs = { mail: PLACEHOLDER_KEY };

/** The KMS outputs a rehearsal binds, whose local custody envelopes nothing. */
const PLACEHOLDER_KMS = { envelope: { kind: "local" }, access: PLACEHOLDER_KEY } as const;

/** The loopback interface the local stage's processes listen on. */
const LOOPBACK = "127.0.0.1";

/** The domain the cloud machines answer below, each one label as `<machine>--platform`. */
const MACHINE_DOMAIN = "destack.computer";

/** The provider running the cloud machines. */
const PROVIDER = "cloudflare";

/** The local universe's loopback port, its issuer's, which the stage's other ports follow. */
const UNIVERSE_PORT = Number(new URL(Stage.of("local").issuer).port);

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

    /** The account service the stage signs people in with, the universe's origin. */
    get issuer(): string {
        return this.stage.name === "local"
            ? `http://${LOOPBACK}:${this.layout.universe}`
            : this.stage.issuer;
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
            const exported = this.stage.name === "local" ? "RESIDENCY_STACK" : residency.stack;
            homes[residency.code] = await build.pinned(exported, {
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
        return this.stage.name === "local" ? "local" : residency.jurisdiction;
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

    /** Bundle the universe and every cloud machine of the stage, deploying nothing. */
    async rehearse(print: (name: string, line: string) => void): Promise<void> {
        // declare the universe over keys in the clear with the stacks built from the checkout
        const { environment } = this;
        await using build = await this.#build();
        const custody = await Custody.local(this.#machineIds(), environment);
        const declaration = await this.universe(
            REHEARSAL_OPERATOR,
            custody,
            build.stacks(this.stage.tag),
            await this.homes(build),
        );

        // configure every script over placeholder outputs
        const scripts = this.#scripts(this.#placeholderUniverse(REHEARSAL_DATABASE), (residency) =>
            this.#placeholderResidency(residency),
        );
        const modules = await Deployment.#modules(build, declaration);

        // build each upload as wrangler deploys it, with the secrets it reads
        for (const [script, configuration] of scripts) {
            const path = await script.write(configuration, modules);
            const secrets = script.secrets(custody, undefined, PLACEHOLDER_SHARED);
            print("rehearse", `${script.name} reads ${Object.keys(secrets).join(", ")}`);
            await Root.execute(
                resolve(ROOT, "node_modules/.bin/wrangler"),
                ["deploy", "--dry-run", "--outdir", resolve(path, "..", "dist"), "--config", path],
                {
                    ...Root.inherited(environment.process.variables, WRANGLER_VARIABLES),
                    WRANGLER_SEND_METRICS: "false",
                },
                environment,
            );
        }
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

        // migrate it as a release migrates the universe's
        const url = `${server}/${database}`;
        await using connection = await connectPostgres(url, Universe.database, {
            environment,
            schema: UNIVERSE_SCHEMA,
        });
        await connection.migrate(Universe.database.tables);

        // connect local Hyperdrive with the password it requires
        const hyperdrive = new URL(url);
        hyperdrive.password ||= HYPERDRIVE_PASSWORD;

        // declare the universe over keys in the clear with the stacks built from the checkout
        await using build = await this.#build();
        const custody = await Custody.local(this.#machineIds(), environment);
        const declaration = await this.universe(
            REHEARSAL_OPERATOR,
            custody,
            build.stacks(this.stage.tag),
            await this.homes(build),
        );

        // configure the universe and the cloud machines over placeholder outputs and the fresh database
        const scripts = this.#scripts(this.#placeholderUniverse(url), (residency) =>
            this.#placeholderResidency(residency),
        ).filter(([script]) => script.target.kind !== "tail");
        const modules = await Deployment.#modules(build, declaration);

        // serve each script on its own ports with placeholder secrets and its root key, and check it answers
        await using children = new AsyncDisposableStack();
        for (const [script, configuration] of scripts) {
            // write the configuration without what only a deployment binds
            const local = Object.fromEntries(
                Object.entries(configuration).filter(([key]) => !DEPLOYED_KEYS.has(key)),
            );
            const path = await script.write(local, modules);
            const secrets = script.secrets(custody, undefined, PLACEHOLDER_SHARED);
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
    }

    /** Apply the shared root, the universe's and the residencies', then release the Workers. */
    async deploy(print: (name: string, line: string) => void): Promise<void> {
        // deploy the release feed's Worker before the shared root routes every download to it
        const { environment } = this;
        const feed = new WorkerScript(this, { kind: "feed" });
        await feed.deploy(feed.feed(), {}, {});

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
        const { shared, outputs, outputsOf } = await this.#outputs();
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

        // migrate the universe's database
        await using connection = await connectPostgres(outputs.database, Universe.database, {
            environment,
            schema: UNIVERSE_SCHEMA,
        });
        await connection.migrate(Universe.database.tables);

        // ship the releases to each cloud machine before it serves them
        for (const residency of RESIDENCIES) {
            const machine = new WorkerScript(this, { kind: "machine", residency });
            await build.writeBucket(machine.bucket(outputsOf(residency), residency));
            print("release", `${machine.name} ships ${build.readers.length} releases`);
        }

        // deploy every script in order, the universe first and each tail before its machine
        const scripts = this.#scripts(outputs, outputsOf);
        const modules = await Deployment.#modules(build, declaration);
        for (const [script, configuration] of scripts) {
            const kms =
                script.target.kind === "universe"
                    ? outputs.kms.access
                    : script.target.kind === "machine"
                      ? outputsOf(script.target.residency).kms.access
                      : undefined;
            await script.deploy(configuration, script.secrets(custody, kms, shared), modules);
        }

        // check every deployed origin answers, a tail answering none
        for (const [script] of scripts) {
            if (script.target.kind !== "tail") {
                await script.smoke();
            }
        }
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

    /** Read the applied shared, universe and residency roots' outputs. */
    async #outputs(): Promise<{
        readonly shared: SharedOutputs;
        readonly outputs: UniverseOutputs;
        readonly outputsOf: (residency: PlatformResidency) => ResidencyOutputs;
    }> {
        // read the shared and the universe root's outputs
        const { environment } = this;
        const sharedRoot = new Root(["shared"], environment);
        const shared = await sharedRoot.shared(await sharedRoot.initialize());
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

        return { shared, outputs, outputsOf };
    }

    /** List the KMS envelopes the custody record seals under: the universe's and each machine's. */
    static #envelopes(
        outputs: UniverseOutputs,
        outputsOf: (residency: PlatformResidency) => ResidencyOutputs,
    ): CustodyEnvelopes {
        return {
            universe: outputs.kms.envelope,
            machines: new Map(
                RESIDENCIES.map((residency) => [
                    residency.machine.id,
                    outputsOf(residency).kms.envelope,
                ]),
            ),
        };
    }

    /** Declare the platform's domains and its residency accounts' hostnames, none locally. */
    #edge(): Pick<PlatformDefinition, "domains" | "hostnames"> {
        // declare no public names locally
        if (this.stage.name === "local") {
            return {
                domains: [],
                hostnames: Object.fromEntries(RESIDENCIES.map((residency) => [residency.code, []])),
            };
        }

        // declare the space and machine domains, each relay's host, and the edge's names at the platform's
        const edge = [`*.${DOMAINS.space}`, `*.${DOMAINS.machine}`];
        const hostnames = RESIDENCIES.map((residency): [string, string[]] => [
            residency.code,
            [
                `relay-${residency.code}.${DOMAINS.machine}`,
                ...(residency.code === PLATFORM.residencyId ? edge : []),
            ],
        ]);

        return {
            domains: [DOMAINS.space, DOMAINS.machine],
            hostnames: Object.fromEntries(hostnames),
        };
    }

    /** List the cloud machines of every residency. */
    #machineIds(): string[] {
        return RESIDENCIES.map((residency) => residency.machine.id);
    }

    /** Build the checkout's first-party releases into the stage's store. */
    async #build(): Promise<PlatformBuild> {
        return await PlatformBuild.build(
            REPOSITORY,
            resolve(REPOSITORY, "target", this.stage.name, "releases"),
            this.environment,
        );
    }

    /** Write the universe root's outputs a rehearsal binds, over a database. */
    #placeholderUniverse(database: string): UniverseOutputs {
        return {
            account: REHEARSAL_IDENTIFIER,
            hyperdrive: REHEARSAL_IDENTIFIER,
            history: `destack-${this.stage.name}-history`,
            database,
            kms: PLACEHOLDER_KMS,
        };
    }

    /** Write a residency root's outputs a rehearsal binds. */
    #placeholderResidency(residency: PlatformResidency): ResidencyOutputs {
        const host = new URL(this.machineOrigin(residency)).host;

        return {
            account: REHEARSAL_IDENTIFIER,
            files: `destack-${this.stage.name}-files-${residency.code}`,
            packages: `destack-${this.stage.name}-packages-${residency.code}`,
            host,
            s3: `s3--${host}`,
            r2: PLACEHOLDER_KEY,
            kms: PLACEHOLDER_KMS,
        };
    }

    /** Supply the scripts' bundles with the build's distribution and the universe's declaration. */
    static async #modules(
        build: PlatformBuild,
        declaration: UniverseDefinition,
    ): Promise<SuppliedModules> {
        return {
            [DISTRIBUTION_MODULE]: await build.distribution(),
            [UNIVERSE_MODULE]: declaration,
        };
    }

    /** Configure the scripts in deploy order: the universe, then each tail and machine. */
    #scripts(
        universe: UniverseOutputs,
        residency: (residency: PlatformResidency) => ResidencyOutputs,
    ): (readonly [WorkerScript, WranglerConfiguration])[] {
        // configure the universe binding its declaration
        const script = new WorkerScript(this, { kind: "universe" });
        const platform = present(
            RESIDENCIES.find((each) => each.code === PLATFORM.residencyId),
            "the platform residency",
        );
        const scripts = [[script, script.universe(universe, residency(platform))] as const];

        // configure each residency's tail before the cloud machine it reads the traces of
        for (const served of RESIDENCIES) {
            const tail = new WorkerScript(this, { kind: "tail", residency: served });
            const machine = new WorkerScript(this, { kind: "machine", residency: served });
            scripts.push(
                [tail, tail.tail(served)],
                [machine, machine.machine(residency(served), served)],
            );
        }

        return scripts;
    }
}

/** Read an AWS session token of temporary credentials, none when unset. */
function sessionToken(token: string | undefined): { readonly sessionToken?: string } {
    return token === undefined || token === "" ? {} : { sessionToken: token };
}
