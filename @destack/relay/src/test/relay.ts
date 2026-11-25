import { ReplicaSet, Scope, type Subject } from "@destack/sync";
import { Caller, principal } from "@destack/access";
import { machine, Region } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import type { DatabaseConnection } from "@destack/db/database";
import { connectBunSqlite } from "@destack/db/bun";
import type { Environment, Listener } from "@destack/environment";
import { PackageMount } from "@destack/object";
import { ObjectConnection, type ObjectRemote } from "@destack/object/client";
import { HibernationHost, PARTITION_HEADER } from "@destack/object/workload";
import { present } from "@destack/schema";
import type { TestMachine, TestUniverse } from "@destack/universe/test";
import relayPackage from "../package.ts";
import { ProcessRelay, TUNNELS_PATH } from "../server/index.ts";

/** The port HTTPS names reach the relay at. */
const HTTPS_PORT = 443;

/** What a test relay is served with: its host, its node's environment and its relays' number. */
export interface TestRelayOptions {
    /** The universe whose names the relay routes. */
    readonly universe: TestUniverse;
    /** The region's managed machine hosting the relay. */
    readonly host: TestMachine;
    /** The clock and server of the relay's node. */
    readonly environment: Environment<"server">;
    /** The host name the relay answers at, where machines open their tunnels. */
    readonly hostname: string;
    /** The names DNS sends to the relay over HTTPS, such as `*.destack.computer`, or none. */
    readonly names?: readonly string[];
}

/** A region's relay run in process by its managed machine, behind the machine's front door. */
export class TestRelay implements AsyncDisposable {
    /** The machine hosting the relay, which its tunnel calls come from. */
    readonly principal: Subject;
    /** The origin the relay answers at. */
    readonly origin: string;
    /** The universe whose rows the relay follows. */
    readonly #universe: TestUniverse;
    /** The relay the machine runs in process. */
    readonly #relay: ProcessRelay;
    /** The memory databases of the instances, the root's under no partition. */
    readonly #memories: ReadonlyMap<string | undefined, Promise<DatabaseConnection>>;
    /** The front door's listeners: the relay's origin and the names DNS sends it. */
    readonly #listeners: readonly Pick<Listener, "stop">[];
    /** The connection to the universe's account store the replicas follow through. */
    readonly #accounts: ObjectConnection;
    /** The stop once asked for, which every later disposal waits for. */
    #stopping: Promise<void> | undefined;

    /** Keep a started relay and its listeners. */
    private constructor(fields: {
        readonly universe: TestUniverse;
        readonly principal: Subject;
        readonly relay: ProcessRelay;
        readonly memories: ReadonlyMap<string | undefined, Promise<DatabaseConnection>>;
        readonly accounts: ObjectConnection;
        readonly listeners: readonly Pick<Listener, "stop">[];
    }) {
        // keep the universe, the hosting machine and the relay
        this.#universe = fields.universe;
        this.principal = fields.principal;
        this.origin = fields.relay.host.origin;
        this.#relay = fields.relay;
        this.#memories = fields.memories;
        this.#accounts = fields.accounts;
        this.#listeners = fields.listeners;
    }

    /** Run a region's relay on its managed machine, its universe rows followed as the machine. */
    static async open(options: TestRelayOptions): Promise<TestRelay> {
        // read the hosting machine's region
        const { universe, host, environment, hostname } = options;
        const directory = universe.storage.database.snapshot();
        const hosted = present(
            await directory.read(machine.table).findFirst({ where: { id: host.id } }),
            `the machine ${host.id}`,
        );
        const { code } = await Region.require(directory, present(hosted.regionId, "a region"));
        const hosting = principal.machine.reference(Scope.universe.id, host.id);

        // follow the universe's rows as the machine follows them
        const accounts = ObjectConnection.open({
            url: ObjectConnection.url(PackageMount.url(host.universe.origin, accountPackage.id)),
            authorize: host.authorize(accountPackage.id),
            environment,
        });
        const universal = universe.universe.store.remote;
        const replicas: ObjectRemote = {
            ...universal,
            async *stream(subscription, signal) {
                yield* accounts.stream({ ...subscription, target: Scope.universe.id }, signal);
            },
            request: async (subscription) => {
                await accounts.request({ ...subscription, target: Scope.universe.id });
            },
            // run the relay's changes at the universe as the hosting machine
            push: (scope, push, signal) =>
                universal.push(
                    scope,
                    { ...push, signer: push.signer ?? Caller.of(hosting) },
                    signal,
                ),
        };

        // keep each instance's memory in its own database, the root's read for its replicas' progress
        const memories = new Map<string | undefined, Promise<DatabaseConnection>>();
        const memory = (partition: string | undefined) => ({
            open: (tables: Parameters<typeof connectBunSqlite>[1]) => {
                const opened = connectBunSqlite(":memory:", tables, { environment });
                memories.set(partition, opened);

                return opened;
            },
        });

        // host the relay as the machine does, its sockets delivered to the tunnels that accepted them
        const sockets = new HibernationHost(async (instanceId, path, request) => {
            await relay.receive(instanceId, new Request(`http://relay${path}`, request));

            return new Response(null, { status: 204 });
        }, environment.telemetry);
        const relay: ProcessRelay = new ProcessRelay({
            origin: `http://${hostname}`,
            region: code,
            universe: host.universe,
            principal: hosting,
            publisher: () => replicas,
            memory,
            hibernation: sockets,
            environment,
        });

        // answer the relay's origin, and the names DNS sends it
        const listening = sockets.listen((request) => relay.fetch(request));
        const { server } = environment;
        const listeners = [
            server.http({ hostname, port: 80, ...listening }),
            ...(options.names ?? []).map((name) =>
                server.http({ hostname: name, port: HTTPS_PORT, ...listening }),
            ),
        ];

        // start the root at once
        await relay.start();

        return new TestRelay({
            universe,
            principal: hosting,
            relay,
            memories,
            accounts,
            listeners,
        });
    }

    /** The URL machines open their tunnels at. */
    get url(): string {
        return `${this.origin}/tunnel`;
    }

    /** The partitions started so far, the root's as absent. */
    get partitions(): readonly (string | undefined)[] {
        return this.#relay.partitions;
    }

    /** Call the relay's tunnels as its hosting machine, in a partition or at the root. */
    itself(path: string, request: RequestInit, partition?: string): Promise<Response> {
        // address the partition, else the root
        const headers = new Headers(request.headers);
        if (partition !== undefined) {
            headers.set(PARTITION_HEADER, partition);
        }
        const tunnels = `${PackageMount.path(relayPackage.id)}${TUNNELS_PATH}`;

        return this.#relay.tunnels(
            new Request(`http://tunnels${tunnels}${path}`, { ...request, headers }),
        );
    }

    /** Open the root's memory, its replicas of the universe. */
    async memory(): Promise<DatabaseConnection> {
        return await present(this.#memories.get(undefined), "the root's memory");
    }

    /** Report whether the root's replicas reached the universe's log as of now. */
    async isCaughtUp(): Promise<boolean> {
        // read the universe's head and the root's replicas of the universe
        const position = await this.#universe.storage.database.log.position();
        const root = await this.memory();
        const replicas = await new ReplicaSet(root).including(Scope.universe.id);

        return await new ReplicaSet(root).isAt(replicas, position);
    }

    /** Wait until the root's replicas reach the universe's log as of now. */
    async waitCaughtUp(signal: AbortSignal): Promise<boolean> {
        // wait until the root's replicas of the universe reach its head
        const position = await this.#universe.storage.database.log.position();
        const root = await this.memory();

        return await new ReplicaSet(root).wait(Scope.universe.id, position, signal);
    }

    /** Stop the instances, take a step meanwhile, then start them again as their host does. */
    restart(meanwhile: () => Promise<void>): Promise<void> {
        return this.#relay.restart(meanwhile);
    }

    /** Stop listening, ending every tunnel, then the instances, once. */
    [Symbol.asyncDispose](): Promise<void> {
        this.#stopping ??= this.#stop();

        return this.#stopping;
    }

    /** Stop listening, ending every tunnel, then the instances and their account connection. */
    async #stop(): Promise<void> {
        // stop listening, ending every tunnel
        for (const listener of this.#listeners) {
            await listener.stop(true);
        }

        // stop the instances, then close the connection they followed through
        await this.#relay[Symbol.asyncDispose]();
        this.#accounts.close();
    }
}
