import { FORGE_INSTALLATION } from "@destack/forge/package";
import { join } from "node:path";
import { MachineKey } from "@destack/account/client";
import { DESKTOP_OBJECTS } from "@destack/desktop/object";
import { KeyPair, type Keychain, LocalKeyring, MemoryKeychain } from "@destack/identity";
import {
    Computer,
    type ComputerOptions,
    MachineProcess,
    MACHINE_KEYS,
    ROOT_KEYS,
} from "@destack/machine/computer";
import type { MachineEnrollment } from "@destack/machine/device";
import { PrintTransport } from "@destack/mail/print";
import { present } from "@destack/schema";
import { Stage } from "@destack/update/release";
import { Toolchain } from "@destack/check/toolchain";
import { BunUniverse } from "./universe.ts";
import { Custody, PlatformBuild } from "../deployment/index.ts";
import { DISTRIBUTION_LAYER, DistributionManifest, DistributionPlatform } from "@destack/package";
import { BunDistribution } from "@destack/package/bun";
import { BuildLayer } from "@destack/package/manifest";
import { Deployment } from "../deployment/index.ts";
import { PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { PrintedMail } from "./mail.ts";
import type { LocalState } from "./state.ts";
import { Operator } from "./operator.ts";

/** The local state. */
const STAGE = Stage.of("local");

/** The root keys of the local universe, which only this computer uses. */
const UNIVERSE_ROOT_KEY =
    '{"active":1,"keys":{"1":"DHDaqs60kZjIDOmpchcC6lggMf8L+sHPgayD5fqCuN0="}}';

/** The root keys of the local cloud machines, which only this computer uses, by residency code. */
const MACHINE_ROOT_KEYS: ReadonlyMap<string, string> = new Map([
    ["eu", '{"active":1,"keys":{"1":"TO1o+AlghKysP7TyW2QfPhOJOOBa/NRh7/a02fsCgQs="}}'],
    ["us", '{"active":1,"keys":{"1":"w20+AFE9NraWIlc9hkKZfGfhO062wFMYkoG5HJo8K2o="}}'],
]);

/** How long a started cloud machine may take to replicate its enabled status, in ms. */
const MACHINE_SERVE_MILLISECONDS = 5_000;

/** A process of the local universe and the URL it answers at. */
export interface DevelopmentEndpoint {
    /** The process's name, such as universe or eu-central. */
    readonly name: string;
    /** The URL it answers at. */
    readonly url: string;
}

/** The local universe of `just local`. */
export class LocalUniverse implements AsyncDisposable {
    /** The processes and the URLs they answer at, in start order. */
    readonly endpoints: readonly DevelopmentEndpoint[];
    /** The local stage's deployment on its loopback layout. */
    readonly deployment: Deployment;
    /** Close what started, last started first. */
    readonly #stack: AsyncDisposableStack;

    /** Start from a universe and what stops it. */
    private constructor(
        endpoints: readonly DevelopmentEndpoint[],
        deployment: Deployment,
        stack: AsyncDisposableStack,
    ) {
        // keep the endpoints, the deployment and the stack
        this.endpoints = endpoints;
        this.deployment = deployment;
        this.#stack = stack;
    }

    /** Create the database, and start the universe and its cloud machines. */
    static async start(state: LocalState): Promise<LocalUniverse> {
        // create the database once
        const local = Deployment.loopback(state.layout, state.environment);
        await using stack = new AsyncDisposableStack();
        await state.createDatabase();

        // build the local releases from the checkout
        const build = stack.use(
            await PlatformBuild.build(state.root, state.environment, { isPrerelease: true }),
        );
        state.scope.log.info("platform.release.built", {
            "platform.release.count": build.readers.length,
        });

        // list the releases the universe ships, with their locks on the host
        const target = DistributionPlatform.of(state.environment.host);
        const shipped = DistributionManifest.releases(build.distribution(target), target);

        // start the universe as its declaration declares it over the fixed local keys, printing its mail
        const mail = new PrintedMail();
        const custody = await LocalUniverse.custody(state.environment);
        const declaration = await local.universe(
            Operator.LOCAL,
            custody,
            build.stacks(STAGE.tag),
            await local.homes(build),
        );
        // serve the origin's other paths from the platform residency's machine once it started, as the edge does
        const clouds = new Map<string, Computer>();
        const edge = async (request: Request) => {
            // answer with the platform residency's machine once it started, at the universe's domain its routes match
            const platform = clouds.get(PLATFORM.residencyId);

            return platform === undefined
                ? new Response("the platform residency's machine is still starting", {
                      status: 503,
                      headers: { "retry-after": "1" },
                  })
                : await platform.fetch(local.edge(request));
        };
        const universe = stack.use(
            await BunUniverse.start({
                edge,
                environment: state.environment,
                definition: declaration,
                origin: local.universeOrigin,
                database: `${state.server}/${state.database}`,
                rootKey: UNIVERSE_ROOT_KEY,
                providers: local.providers(
                    (id) => state.environment.process.variables[Deployment.providerSecret(id)],
                ),
                mail: new PrintTransport((text) => {
                    mail.push(text);
                    state.print("mail", text);
                }),
                directory: join(state.directory, "universe"),
                report: (error) => Computer.capture(environment.telemetry, "universe", error),
                shipped: {
                    releases: shipped,
                    open: (lock) => build.store.root(lock),
                },
            }),
        );
        const endpoints: DevelopmentEndpoint[] = [{ name: "universe", url: universe.url.origin }];

        // report the universe
        state.scope.log.info("platform.universe.started", {
            "url.full": universe.url.origin,
        });

        // write the distribution once, then start each residency's cloud machine shipping it
        await state.writeDistribution(build);
        const directory = state.distribution;
        const { environment } = build.store;
        const distribution = {
            shipped: await BunDistribution.read(directory, environment.files),
            layer: await BuildLayer.open(join(directory, DISTRIBUTION_LAYER), environment),
        };
        state.scope.log.info("platform.distribution.written", {
            "file.path": directory,
        });
        for (const residency of RESIDENCIES) {
            const started = await LocalUniverse.#startCloud(
                residency,
                distribution,
                local,
                custody,
                state,
            );
            stack.use(started.stack);
            clouds.set(residency.code, started.machine);

            // wait until it serves, refusing at the bound with why it does not
            const window = state.environment.clock.timeout(MACHINE_SERVE_MILLISECONDS);
            await started.machine.machine.health.waitUp(window);
            state.scope.log.info("platform.machine.started", {
                "platform.machine.name": residency.machine.name,
                "url.full": local.machineOrigin(residency),
            });
            endpoints.push({
                name: residency.machine.name,
                url: local.machineOrigin(residency),
            });
        }

        // publish the releases into the forge as the local operator
        await local.publishLocal(build, custody, mail);

        return new LocalUniverse(endpoints, local, stack.move());
    }

    /** Stop the local stage's machine, the cloud machines and the universe, last started first. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#stack.disposeAsync();
    }

    /** Read the root keys of a residency's local cloud machine. */
    static #rootKey(residency: PlatformResidency): string {
        return present(MACHINE_ROOT_KEYS.get(residency.code), `the root keys of ${residency.code}`);
    }

    /** Keep the fixed local root keys in the clear with each cloud machine's key they derive. */
    static async custody(environment: LocalState["environment"]): Promise<Custody> {
        const machines = await Promise.all(
            RESIDENCIES.map(async (residency) => {
                // derive the machine's key from its fixed root keys
                const { id } = residency.machine;
                const rootKey = LocalUniverse.#rootKey(residency);
                const keychain = new MemoryKeychain();
                await MachineKey.save(id, await LocalKeyring.read(rootKey, environment), keychain);
                const key = present(await keychain.load(id), `the key of ${id}`);
                const { publicKey } = await KeyPair.read(key, environment.crypto);

                return [id, { envelope: { kind: "local" }, rootKey, publicKey, key }] as const;
            }),
        );

        return new Custody({
            universe: { envelope: { kind: "local" }, rootKey: UNIVERSE_ROOT_KEY },
            machines: Object.fromEntries(machines),
        });
    }

    /** Keep a residency cloud machine's root keys and its key in memory. */
    static async #keychain(
        residency: PlatformResidency,
        custody: Custody,
    ): Promise<(service: string) => Keychain> {
        // open one memory keychain per service
        const keychains = new Map<string, MemoryKeychain>();
        const keychain = (service: string) => {
            const kept = keychains.get(service) ?? new MemoryKeychain();
            keychains.set(service, kept);

            return kept;
        };

        // keep the root keys and the machine key of its custody
        const { id } = residency.machine;
        const keys = custody.machine(id);
        await keychain(ROOT_KEYS).save(id, keys.rootKey);
        await keychain(MACHINE_KEYS).save(id, keys.key);

        return keychain;
    }

    /** Read the enrollment the universe declares for a residency's cloud machine. */
    static #enrollment(residency: PlatformResidency, local: Deployment): MachineEnrollment {
        return {
            kind: "cloud",
            universe: local.universeOrigin,
            residencyId: residency.code,
            status: "enrolled",
        };
    }

    /** Start a residency's cloud machine in this process, with the stack stopping it. */
    static async #startCloud(
        residency: PlatformResidency,
        distribution: Pick<ComputerOptions, "shipped" | "layer">,
        local: Deployment,
        custody: Custody,
        state: LocalState,
    ): Promise<{ readonly machine: Computer; readonly stack: AsyncDisposableStack }> {
        // read the machine's root keys and its key
        await using stack = new AsyncDisposableStack();
        const { id } = residency.machine;
        const keychain = await LocalUniverse.#keychain(residency, custody);

        // listen at its origin first, answering as unavailable until it started, as the universe and other machines call it while it starts
        let started: Computer | undefined;
        const { environment } = state;
        const origin = new URL(local.machineOrigin(residency));
        const listener = environment.server.http({
            hostname: origin.hostname,
            port: Number(origin.port),
            fetch: (request) =>
                started === undefined
                    ? new Response(null, { status: 503 })
                    : started.fetch(request),
        });
        stack.defer(async () => listener.stop(true));

        // start it enrolled in the universe as the declared cloud machine of the residency, in a directory of its own
        const name = residency.machine.name;
        const now = environment.clock.now();
        const directory = join(state.directory, "machines", name);
        await new MachineProcess(directory, environment).createDirectory();
        const machine = stack.use(
            await Computer.start({
                isolation: "sandbox",
                registry: FORGE_INSTALLATION,
                directory: await environment.files.realPath(directory),
                universe: local.universeOrigin,
                port: Number(origin.port),
                environment,
                localObjects: DESKTOP_OBJECTS,
                ...distribution,
                keychain,
                toolchain: async () => Toolchain.workspace(),
                machineId: id,
                enrollment: LocalUniverse.#enrollment(residency, local),
                process: {
                    version: STAGE.tag,
                    pid: environment.process.id,
                    startedAt: now,
                },
                shutdown: () =>
                    state.scope.log.info("platform.machine.signaled", {
                        "platform.machine.name": name,
                    }),
            }),
        );

        // serve its origin
        started = machine;

        return { machine, stack: stack.move() };
    }
}
