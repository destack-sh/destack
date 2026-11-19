import { FORGE_INSTALLATION } from "@destack/forge/package";
import { join } from "node:path";
import { DESKTOP_OBJECTS } from "@destack/desktop/object";
import { type Keychain, MemoryKeychain } from "@destack/identity";
import {
    Computer,
    type ComputerOptions,
    MachineProcess,
    MACHINE_KEYS,
    ROOT_KEYS,
} from "@destack/machine/computer";
import type { MachineEnrollment } from "@destack/machine/device";
import { PrintTransport } from "@destack/mail/print";
import { Universe } from "@destack/update/release";
import { Toolchain } from "@destack/check/toolchain";
import { BunUniverse } from "./universe.ts";
import type { HostListening } from "@destack/object/workload";
import { present } from "@destack/schema";
import { Custody, PlatformBuild } from "../deployment/index.ts";
import { DISTRIBUTION_LAYER, DistributionManifest, DistributionPlatform } from "@destack/package";
import { BunDistribution } from "@destack/package/bun";
import { BuildLayer } from "@destack/package/manifest";
import { Deployment } from "../deployment/index.ts";
import { PLATFORM, type PlatformResidency, RESIDENCIES } from "@platform/stack";
import { PrintedMail } from "./mail.ts";
import type { LocalUniverse } from "./local.ts";
import { Operator } from "./operator.ts";

/** The local universe's identity, whose tag its releases publish under. */
const LOCAL = Universe.of("local");

/** How long a started cloud machine may take to replicate its enabled status, in ms. */
const MACHINE_SERVE_MILLISECONDS = 5_000;

/** A process of the local universe and the URL it answers at. */
export interface DevelopmentEndpoint {
    /** The process's name, such as universe or eu-central. */
    readonly name: string;
    /** The URL it answers at. */
    readonly url: string;
}

/** The universe and the cloud machines of a local universe, run on Bun in this process. */
export class BunCloud implements AsyncDisposable {
    /** The processes and the URLs they answer at, in start order. */
    readonly endpoints: readonly DevelopmentEndpoint[];
    /** The local universe's deployment on its loopback layout. */
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
    static async start(local: LocalUniverse): Promise<BunCloud> {
        // create the database once
        const deployment = Deployment.loopback(local.layout, local.environment);
        await using stack = new AsyncDisposableStack();
        await local.createDatabase();

        // build the deployment releases from the checkout
        const build = stack.use(
            await PlatformBuild.build(local.root, local.environment, { isPrerelease: true }),
        );

        // list the releases the universe ships, with their images on the host
        const target = DistributionPlatform.of(local.environment.host);
        const shipped = DistributionManifest.releases(build.distribution(target), target);

        // start the universe as its declaration declares it over the fixed deployment keys, printing its mail
        const mail = new PrintedMail();
        const custody = await local.custody();
        const declaration = await deployment.definition(
            Operator.LOCAL,
            custody,
            build.stacks(LOCAL.tag),
            await deployment.homes(build),
        );
        // serve the origin's other paths from the platform residency's machine once it started, as the edge does
        const clouds = new Map<string, Computer>();
        const edge = BunCloud.#listen(
            () => clouds.get(PLATFORM.residencyId),
            (request) => deployment.edge(request),
        );
        const universe = stack.use(
            await BunUniverse.start({
                edge,
                environment: local.environment,
                definition: declaration,
                origin: deployment.universe.origin,
                database: `${local.server}/${local.database}`,
                rootKey: custody.record.universe.rootKey,
                providers: deployment.providers(
                    (id) => local.environment.process.variables[Deployment.providerSecret(id)],
                ),
                mail: new PrintTransport((text) => {
                    mail.push(text);
                    local.print("mail", text);
                }),
                directory: join(local.directory, "universe"),
                report: (error) => Computer.capture(environment.telemetry, "universe", error),
                shipped: {
                    releases: shipped,
                    open: (image) => build.store.root(image),
                },
            }),
        );
        const endpoints: DevelopmentEndpoint[] = [{ name: "universe", url: universe.url.origin }];

        // report the universe
        local.scope.log.info("platform.universe.started", {
            "url.full": universe.url.origin,
        });

        // write the distribution once, then start each residency's cloud machine shipping it
        await local.writeDistribution(build);
        const directory = local.distribution;
        const { environment } = build.store;
        const distribution = {
            shipped: await BunDistribution.read(directory, environment.files),
            layer: await BuildLayer.open(join(directory, DISTRIBUTION_LAYER), environment),
        };
        local.scope.log.info("platform.distribution.written", {
            "file.path": directory,
        });
        for (const residency of RESIDENCIES) {
            const started = await BunCloud.#startCloud(
                residency,
                distribution,
                deployment,
                custody,
                local,
            );
            stack.use(started.stack);
            clouds.set(residency.code, started.machine);

            // wait until it serves, refusing at the bound with why it does not
            const window = local.environment.clock.timeout(MACHINE_SERVE_MILLISECONDS);
            await started.machine.machine.health.waitUp(window);
            local.scope.log.info("platform.machine.started", {
                "platform.machine.name": residency.machine.name,
                "url.full": deployment.machineOrigin(residency),
            });
            endpoints.push({
                name: residency.machine.name,
                url: deployment.machineOrigin(residency),
            });
        }

        // publish the releases into the forge as the deployment operator
        await deployment.publishLocal(build, custody, mail);

        return new BunCloud(endpoints, deployment, stack.move());
    }

    /** Stop the machines and the universe, last started first. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#stack.disposeAsync();
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
    static #enrollment(residency: PlatformResidency, deployment: Deployment): MachineEnrollment {
        return {
            kind: "cloud",
            universe: deployment.universe.origin,
            residencyId: residency.code,
            status: "enrolled",
        };
    }

    /** Start a residency's cloud machine in this process, with the stack stopping it. */
    static async #startCloud(
        residency: PlatformResidency,
        distribution: Pick<ComputerOptions, "shipped" | "layer">,
        deployment: Deployment,
        custody: Custody,
        local: LocalUniverse,
    ): Promise<{ readonly machine: Computer; readonly stack: AsyncDisposableStack }> {
        // read the machine's root keys and its key
        await using stack = new AsyncDisposableStack();
        const { id } = residency.machine;
        const keychain = await BunCloud.#keychain(residency, custody);

        // listen at its origin first, answering as unavailable until it started, as the universe and other machines call it while it starts
        let started: Computer | undefined;
        const { environment } = local;
        const origin = new URL(deployment.machineOrigin(residency));
        const listening = BunCloud.#listen(
            () => started,
            (request) => request,
        );
        const listener = environment.server.http({
            hostname: origin.hostname,
            port: Number(origin.port),
            ...listening,
        });
        stack.defer(async () => listener.stop(true));

        // start it enrolled in the universe as the declared cloud machine of the residency, in a directory of its own
        const name = residency.machine.name;
        const now = environment.clock.now();
        const directory = join(local.directory, "machines", name);
        await new MachineProcess(directory, environment).createDirectory();
        const machine = stack.use(
            await Computer.start({
                isolation: "sandbox",
                registry: FORGE_INSTALLATION,
                directory: await environment.files.realPath(directory),
                universe: deployment.universe.origin,
                port: Number(origin.port),
                environment,
                localObjects: DESKTOP_OBJECTS,
                ...distribution,
                keychain,
                toolchain: async () => Toolchain.workspace(),
                machineId: id,
                enrollment: BunCloud.#enrollment(residency, deployment),
                process: {
                    version: LOCAL.tag,
                    pid: environment.process.id,
                    startedAt: now,
                },
                shutdown: () =>
                    local.scope.log.info("platform.machine.signaled", {
                        "platform.machine.name": name,
                    }),
            }),
        );

        // serve its origin
        started = machine;

        return { machine, stack: stack.move() };
    }

    /** Serve a computer's requests and sockets at its host, unavailable until it started. */
    static #listen(
        computer: () => Computer | undefined,
        edge: (request: Request) => Request,
    ): HostListening {
        // listen through the started computer's host, which keeps the sockets its spaces accept
        const listening = () => {
            const started = computer();

            return started === undefined
                ? undefined
                : present(started.machine.hibernation, "the machine's sockets").listen((request) =>
                      started.fetch(request),
                  );
        };
        const sockets = () => present(listening(), "the started machine's listening").websocket;

        return {
            fetch: async (request) => {
                // answer as unavailable while the computer starts, as callers retry
                const started = listening();
                if (started === undefined) {
                    return new Response("the machine is still starting", {
                        status: 503,
                        headers: { "retry-after": "1" },
                    });
                }

                return started.fetch(edge(request));
            },
            // hand the sockets the computer's host accepted, or refused, their events
            websocket: {
                open: (socket) => sockets().open?.(socket),
                message: (socket, message) => sockets().message(socket, message),
                close: (socket, code, reason) => sockets().close?.(socket, code, reason),
            },
        };
    }
}
