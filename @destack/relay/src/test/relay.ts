import { AccessContext, Caller } from "@destack/access";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { machine, Residency } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { type Environment, type Listener } from "@destack/environment";
import { Egress, PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import {
    type EphemeralHost,
    FORWARDED_HEAD_HEADER,
    ForwardedRequest,
    HibernationHost,
    PARTITION_HEADER,
    ROUTE_PATH,
    SOCKET_HEADER,
    SOCKET_INSTANCE_HEADER,
    SOCKET_PATH,
    WorkloadRunner,
} from "@destack/object/workload";
import { Package } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { MemoryBuild } from "@destack/package/test";
import { Identifier, present, schema, Uuid } from "@destack/schema";
import { Replica, Scope, type Subject, type Uplink } from "@destack/sync";
import { TelemetryScope } from "@destack/telemetry";
import type { TestMachine, TestUniverse } from "@destack/universe/test";
import relayPackage from "../package.ts";
import { issuer, origin, relayWorkload, TUNNELS_PATH } from "../server/index.ts";
import { residency } from "../declare/setting.ts";

/** The status of an upgrade the front door failed to complete (RFC 9110 15.5.22). */
const UPGRADE_REQUIRED = 426;

/** The port HTTPS names reach the relay at. */
const HTTPS_PORT = 443;

/** The status of a call to an address the front door does not serve (RFC 9110 15.5.5). */
const NOT_FOUND = 404;

/** The name the relay installation has in its residency's home. */
const NAME = "relay";

/** One started instance of the relay's workload: the root, or a machine's partition. */
interface Instance {
    /** The instance's runner. */
    readonly runner: WorkloadRunner;
    /** The secret the front door and the runner prove each other's requests with. */
    readonly secret: string;
}

/** What a test relay is served with: its host, its node's environment and its relays' number. */
export interface TestRelayOptions {
    /** The universe whose names the relay routes. */
    readonly universe: TestUniverse;
    /** The residency's cloud machine hosting the relay in the residency's home. */
    readonly host: TestMachine;
    /** The clock and server of the relay's node. */
    readonly environment: Environment<"server">;
    /** The host name the relay answers at, where machines open their tunnels. */
    readonly hostname: string;
    /** The relay installation it serves as, a new one in the residency's home by default. */
    readonly installation?: Subject;
    /** The names DNS sends to the relay over HTTPS, such as `*.destack.computer`, or none. */
    readonly names?: readonly string[];
}

/** A relay installation's workload run in process behind its machine's front door. */
export class TestRelay implements AsyncDisposable {
    /** The relay installation in the residency's home space. */
    readonly installation: Subject;
    /** The origin the relay answers at. */
    readonly origin: string;
    /** The universe whose rows the relay follows. */
    readonly #universe: TestUniverse;
    /** The memory databases of the instances, the root's under no partition. */
    readonly #memories: ReadonlyMap<string | undefined, Promise<DatabaseConnection>>;
    /** The instances started so far, the root's under no partition. */
    readonly #instances: Map<string | undefined, Promise<Instance>>;
    /** Start the root's or a partition's instance once. */
    readonly #start: (partition: string | undefined) => Promise<Instance>;
    /** The front door's listeners: the relay's origin and the host's egress. */
    readonly #listeners: readonly Pick<Listener<unknown>, "stop">[];
    /** Call the root or a partition as the installation itself, below its address at the egress. */
    readonly #call: (request: Request) => Promise<Response>;
    /** The URL of the installation's own address at the host's egress. */
    readonly #address: string;
    /** The stop once asked for, which every later disposal waits for. */
    #stopping: Promise<void> | undefined;

    /** Keep a started relay's instances and listeners. */
    private constructor(
        universe: TestUniverse,
        installation: Subject,
        relayOrigin: string,
        running: {
            readonly instances: Map<string | undefined, Promise<Instance>>;
            readonly memories: ReadonlyMap<string | undefined, Promise<DatabaseConnection>>;
            readonly start: (partition: string | undefined) => Promise<Instance>;
            readonly call: (request: Request) => Promise<Response>;
            readonly itself: string;
        },
        listeners: readonly Pick<Listener<unknown>, "stop">[],
    ) {
        // keep the universe, the installation and its origin
        this.#universe = universe;
        this.#memories = running.memories;
        this.installation = installation;
        this.origin = relayOrigin;
        this.#instances = running.instances;
        this.#start = running.start;
        this.#call = running.call;
        this.#address = running.itself;
        this.#listeners = listeners;
    }

    /** Run a relay installation in its host's residency home, its universe rows from the host. */
    static async open(options: TestRelayOptions): Promise<TestRelay> {
        // take the installation in the home of the host's residency
        const { universe, host, environment, hostname } = options;
        const directory = Snapshot.live(universe.storage.database);
        const hosted = present(
            await directory.read(machine.table).findFirst({ where: { id: host.id } }),
            `the machine ${host.id}`,
        );
        const residencyId = present(hosted.residencyId, "a residency");
        const home = await Residency.home(directory, residencyId);
        const installation = options.installation ?? (await universe.relay(residencyId));
        const relayOrigin = `http://${hostname}`;
        const egress = `http://host.${hostname}${Egress.path}`;

        // stand in for the home space's replicas: the universe's rows as its machine follows them
        const accounts = createClient(
            accountPackage,
            {
                url: host.universe.accounts,
                fetch: host.fetch(accountPackage.id),
            },
            environment,
        ).store;
        const replicas: Uplink = {
            async *stream(subscription, signal) {
                const followed = { ...subscription, target: Scope.universe.id };
                yield* await accounts.stream(followed, { signal });
            },
            confirm: async (subscription, watermark) => {
                const followed = { ...subscription, target: Scope.universe.id };
                await accounts.confirm({ subscription: followed, watermark });
            },
            receive: async (mutation, signer) => {
                // run the relay's changes at the universe as their signer, as the home's machine sends them
                const caller = Caller.of(signer ?? installation);

                return universe.universe.store.mutate(
                    AccessContext.of(caller, environment.clock.now()),
                    mutation,
                );
            },
            call: async () => {
                throw new TypeError("the relay asks the universe no queries");
            },
        };

        // keep each instance's memory in its own database, the root's read for its replicas' progress
        const { clock, server, random } = environment;
        const memories = new Map<string | undefined, Promise<DatabaseConnection>>();
        const memory = (partition: string | undefined): EphemeralHost => ({
            open: (tables) => {
                const opened = connectBunSqlite(":memory:", tables, { environment });
                memories.set(partition, opened);

                return opened;
            },
            channel: () => {
                throw new TypeError("a test relay runs one instance per partition");
            },
        });

        // hibernate the tunnels' sockets, delivering their events to the instances that accepted them
        const instances = new Map<string | undefined, Promise<Instance>>();
        const partitions = new Map<string, string | undefined>();
        const sockets = new HibernationHost(async (instanceId, path, request) => {
            // deliver the event to the instance below the socket's path, with its secret
            if (!partitions.has(instanceId)) {
                throw new TypeError(`the test relay started no instance ${instanceId}`);
            }
            const { runner, secret } = await start(partitions.get(instanceId));
            const headers = new Headers(request.headers);
            headers.set("authorization", `Bearer ${secret}`);

            return runner.fetch(new Request(`http://runner${path}`, { ...request, headers }));
        }, environment.telemetry);

        // start the root, or a machine's partition on its first call, as the machine starts them
        const build = await MemoryBuild.write(
            new Map(),
            { package: Package.of(relayPackage) },
            environment.crypto,
        );
        const settings = {
            [DeclarationReference.key(origin.reference)]: relayOrigin,
            [DeclarationReference.key(issuer.reference)]: host.universe.issuer,
            [DeclarationReference.key(residency.reference)]: residencyId,
        };
        const start = (partition: string | undefined): Promise<Instance> => {
            // take a started instance
            const existing = instances.get(partition);
            if (existing !== undefined) {
                return existing;
            }

            // start the workload as a new instance with its own secret
            const instance = TestRelay.#instance(partitions, partition, environment);
            const secret = Uuid.v4(random);
            const started = WorkloadRunner.start(
                {
                    workload: relayWorkload,
                    resources: {},
                    publisher: () => replicas,
                    socketHost: {
                        url: `http://host.${hostname}${SOCKET_PATH}`,
                        fetch: (request) => sockets.answer(instance, request),
                    },
                    ephemeral: memory(partition),
                    environment,
                },
                {
                    root: "00".repeat(32),
                    capabilities: [],
                    instance,
                    scope: home,
                    installation: schema.identifier("installation").parse(installation.id),
                    origin: "declared",
                    name: NAME,
                    ...(partition === undefined ? {} : { partition }),
                    bindings: {},
                    settings,
                    secret,
                    egress,
                    url: relayOrigin,
                    sampling: 0,
                },
                build,
                // record the workload's telemetry in the world, as its node's
                async () => ({ ...environment.telemetry, shutdown: async () => {} }),
                (error) =>
                    TelemetryScope.of(
                        import.meta.destack.package,
                        environment.telemetry,
                    ).captureException(error),
            ).then((runner) => ({ runner, secret }));
            instances.set(partition, started);

            return started;
        };

        // call the root or a partition as the installation itself, below its address at the egress
        const call = async (request: Request): Promise<Response> => {
            // require the installation's own address
            const addressed = Egress.of(request);
            if (addressed?.address !== Egress.address(NAME, home)) {
                return new Response(null, { status: NOT_FOUND });
            }

            // forward it to the partition it names, else the root, as the installation
            const partition = addressed.request.headers.get(PARTITION_HEADER) ?? undefined;
            const target = await start(partition);
            const headers = new Headers(addressed.request.headers);
            headers.set("authorization", `Bearer ${target.secret}`);
            Caller.forward(TestRelay.#itself(installation, clock.now()), headers);
            const url = new URL(addressed.request.url);
            const answer = await target.runner.fetch(
                new Request(`http://runner${url.pathname}${url.search}`, {
                    method: addressed.request.method,
                    headers,
                    body: addressed.request.body,
                }),
            );

            // name the instance that accepted a socket
            if (!answer.headers.has(SOCKET_HEADER)) {
                return answer;
            }
            const accepted = new Headers(answer.headers);
            accepted.set(SOCKET_INSTANCE_HEADER, target.runner.start.instance);

            return new Response(answer.body, { status: answer.status, headers: accepted });
        };
        const egressListener = server.http({
            hostname: `host.${hostname}`,
            port: 80,
            fetch: (request) => call(request),
        });

        // hand names and tunnel openings to the root's route, hibernating the sockets the partitions accept
        const route = async (
            request: Request,
            listening: Parameters<typeof sockets.upgrade>[0],
        ) => {
            // hand the client's head to the root's route at the URL it asked for
            const root = await start(undefined);
            const url = new URL(request.url);
            const authority = request.headers.get("host") ?? url.host;
            const asked = `http://${authority}${url.pathname}${url.search}`;
            const headers = new Headers({
                [FORWARDED_HEAD_HEADER]: ForwardedRequest.head(request, asked),
                authorization: `Bearer ${root.secret}`,
            });
            const answer = await root.runner.fetch(
                new Request(`http://runner${ROUTE_PATH}`, {
                    method: request.method,
                    headers,
                    body: request.body,
                }),
            );

            // hibernate a socket a partition accepted for an upgrade
            if (
                request.headers.get("upgrade") !== "websocket" ||
                !answer.headers.has(SOCKET_HEADER)
            ) {
                return answer;
            }

            return sockets.upgrade(listening, request, answer)
                ? undefined
                : new Response(null, { status: UPGRADE_REQUIRED });
        };
        const routeListener = server.http({
            hostname,
            port: 80,
            fetch: route,
            websocket: sockets.websocket,
        });

        // answer the names DNS sends to the relay as well
        const named = (options.names ?? []).map((name) =>
            server.http({
                hostname: name,
                port: HTTPS_PORT,
                fetch: route,
                websocket: sockets.websocket,
            }),
        );

        // start the root at once
        await start(undefined);

        // keep the installation's own address at the egress
        const itself = Egress.url(egress, Egress.address(NAME, home));

        return new TestRelay(
            universe,
            installation,
            relayOrigin,
            { instances, memories, start, call, itself },
            [routeListener, egressListener, ...named],
        );
    }

    /** The URL machines open their tunnels at. */
    get url(): string {
        return `${this.origin}/tunnel`;
    }

    /** The partitions started so far, the root's as absent. */
    get partitions(): readonly (string | undefined)[] {
        return [...this.#instances.keys()];
    }

    /** Call the relay's tunnels as the installation itself, in a partition or at the root. */
    itself(path: string, request: RequestInit, partition?: string): Promise<Response> {
        // address the partition, else the root
        const headers = new Headers(request.headers);
        if (partition !== undefined) {
            headers.set(PARTITION_HEADER, partition);
        }
        const tunnels = `${PackageMount.path(relayPackage.id)}${TUNNELS_PATH}`;

        return this.#call(
            new Request(`${this.#address}${tunnels}${path}`, { ...request, headers }),
        );
    }

    /** Report whether the root's replicas reached the universe's log as of now. */
    async isSettled(): Promise<boolean> {
        const position = await this.#universe.storage.database.log.position();
        const root = await present(this.#memories.get(undefined), "the root's memory");

        return Replica.isAt(root, Scope.universe.id, position);
    }

    /** Wait until the root's replicas reach the universe's log, at each commit of the root. */
    async settled(signal: AbortSignal): Promise<boolean> {
        const root = await present(this.#memories.get(undefined), "the root's memory");

        return root.log.until(() => this.isSettled(), signal);
    }

    /** Stop the instances, take a step meanwhile, then start them again as their host does. */
    async restart(meanwhile: () => Promise<void>): Promise<void> {
        // stop every instance, keeping the sockets the front door hibernates
        const stopped = [...this.#instances];
        this.#instances.clear();
        for (const [, started] of stopped) {
            await (await started).runner.close();
        }

        // take the step, then start the same instances again
        await meanwhile();
        for (const [partition] of stopped) {
            await this.#start(partition);
        }
    }

    /** Stop listening, ending every tunnel, then the instances, once. */
    [Symbol.asyncDispose](): Promise<void> {
        this.#stopping ??= this.#stop();

        return this.#stopping;
    }

    /** Stop listening, ending every tunnel, then the instances. */
    async #stop(): Promise<void> {
        for (const listener of this.#listeners) {
            await listener.stop(true);
        }
        for (const started of this.#instances.values()) {
            await (await started).runner.close();
        }
    }

    /** Read the instance of a partition, the one it started as before, else a new one. */
    static #instance(
        partitions: Map<string, string | undefined>,
        partition: string | undefined,
        environment: Pick<Environment, "clock" | "random">,
    ) {
        // take the instance the partition started as before
        for (const [instance, started] of partitions) {
            if (started === partition) {
                return schema.identifier("instance").parse(instance);
            }
        }

        // add a new one
        const instance = Identifier.create("instance", environment);
        partitions.set(instance, partition);

        return instance;
    }

    /** Authenticate a call as the relay installation itself. */
    static #itself(installation: Subject, now: number) {
        return {
            caller: Caller.of(installation),
            credential: {
                kind: "installation",
                id: installation.id,
                audience: relayPackage.id,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
    }
}
