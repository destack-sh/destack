import { Caller } from "@destack/access";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import type { UniverseOrigin } from "@destack/account/address";
import type { RegionCode } from "@destack/account/object";
import type { Environment } from "@destack/environment";
import { type Fetch, PackageMount } from "@destack/object";
import {
    CLIENT_HEAD_HEADER,
    ClientHead,
    type EphemeralHost,
    type Hibernation,
    type HibernationHost,
    PARTITION_HEADER,
    ROUTE_HEADER,
    WorkloadInstance,
} from "@destack/object/workload";
import relayPackage from "../package.ts";
import { relayHost, relayWorkload } from "./workload.ts";
import { ResourceContext } from "@destack/resource/context";
import type { Publisher, Subject } from "@destack/sync";

/** The longest drain of a relay instance's calls, in milliseconds. */
const DRAIN_MILLISECONDS = 30_000;

/** The URL a machine calls its own relay's root and tunnels at, which never leaves the machine. */
const TUNNELS_URL = "http://tunnels";

/** What a machine hosts its region's relay with, on every runtime. */
export interface RelayHostOptions {
    /** The relay's origin, its region's, where self-hosted machines open their tunnels. */
    readonly origin: string;
    /** The region the machine serves. */
    readonly region: RegionCode;
    /** The universe the relay's names resolve in. */
    readonly universe: UniverseOrigin;
    /** The machine hosting the relay. */
    readonly principal: Subject;
    /** The universe's publisher of the rows the relay follows, the machine's own. */
    readonly publisher: (scope: string) => Publisher;
    /** Call the root, or the tunnel a call's partition names, where the runtime runs it. */
    readonly tunnels: Fetch;
    /** The machine's environment. */
    readonly environment: Environment;
}

/** The managed machine hosting a region's relay: each instance's host, and calls into them. */
export class RelayHost {
    /** The relay's own origin, where machines open their tunnels. */
    readonly origin: string;
    /** The region whose accounts' machines tunnel to the relay. */
    readonly region: RegionCode;
    /** The universe, whose issuer signs machines' tokens and whose domains names resolve under. */
    readonly universe: UniverseOrigin;
    /** The machine hosting the relay, which follows the universe's rows as itself. */
    readonly principal: Subject;
    /** The universe's publisher of the rows the relay follows. */
    readonly publisher: (scope: string) => Publisher;
    /** Call managed machines at their published endpoints. */
    readonly fetch: Fetch;
    /** Call the relay's root and its machines' tunnels below the tunnels mount. */
    readonly tunnels: { readonly url: string; readonly fetch: Fetch };
    /** The partition of a machine's tunnel this instance runs, absent at the root. */
    readonly partition: string | undefined;
    /** What the machine hosts the relay with. */
    readonly #options: RelayHostOptions;

    /** Host a region's relay on its machine, for the root or a tunnel's partition. */
    constructor(options: RelayHostOptions, partition?: string) {
        // keep what every instance shares, and the partition of this one
        this.origin = options.origin;
        this.region = options.region;
        this.universe = options.universe;
        this.principal = options.principal;
        this.publisher = options.publisher;
        this.fetch = (request) => options.environment.network.fetch(request);
        this.tunnels = { url: TUNNELS_URL, fetch: options.tunnels };
        this.partition = partition;
        this.#options = options;
    }

    /** Read the partition a call into the relay's instances names, absent for the root. */
    static partition(request: Request): string | undefined {
        return request.headers.get(PARTITION_HEADER) ?? undefined;
    }

    /** Start the root's or a tunnel's instance over its memory, a tunnel's sockets hibernated. */
    start(
        partition: string | undefined,
        place: { readonly memory: EphemeralHost; readonly hibernation?: Hibernation },
    ): Promise<WorkloadInstance> {
        return WorkloadInstance.start(relayWorkload, {
            resources: new ResourceContext().bind(
                relayHost,
                new RelayHost(this.#options, partition),
            ),
            callKey: async () => {
                throw new TypeError("the relay journals no calls");
            },
            ephemeral: place.memory,
            ...(place.hibernation === undefined ? {} : { hibernation: place.hibernation }),
            environment: this.#options.environment,
            serverOptions: () => ({
                authenticate: async () => undefined,
                authorizeMachine: async () => {},
                drainTimeout: DRAIN_MILLISECONDS,
            }),
        });
    }

    /** Hand a request the machine does not serve to the root's names, at the URL it asked for. */
    names(root: WorkloadInstance, request: Request): Promise<Response> {
        // read the URL the client asked for, under the host it asked for
        const url = new URL(request.url);
        const host = request.headers.get("host") ?? url.host;
        const asked = `${url.protocol}//${host}${url.pathname}${url.search}`;
        const headers = new Headers({
            [CLIENT_HEAD_HEADER]: ClientHead.write(request, asked),
            [ROUTE_HEADER]: "route-names",
        });

        return root.fetch(
            relayPackage,
            new Request("http://relay/names", {
                method: request.method,
                headers,
                body: request.body,
                signal: request.signal,
                ...(request.body === null ? {} : { duplex: "half" }),
            }),
        );
    }

    /** Call an instance below the relay's package, as the hosting machine. */
    call(target: WorkloadInstance, request: Request): Promise<Response> {
        const url = new URL(request.url);
        url.pathname = url.pathname.slice(PackageMount.path(relayPackage.id).length);

        return target.fetch(relayPackage, new Request(url.href, request), () => this.#vouch());
    }

    /** Vouch for the hosting machine at a call it makes to its own relay. */
    async #vouch() {
        const now = this.#options.environment.clock.now();

        return {
            caller: Caller.of(this.principal),
            credential: {
                kind: "machine",
                id: this.principal.id,
                audience: relayPackage.id,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
    }
}

/** A region's relay a managed machine runs in its process: its root and each machine's tunnel. */
export class ProcessRelay implements AsyncDisposable {
    /** The host the instances start from. */
    readonly host: RelayHost;
    /** The instances started so far, the root's under no partition. */
    readonly #instances = new Map<string | undefined, Promise<WorkloadInstance>>();
    /** Open each instance's memory, the root's under no partition. */
    readonly #memory: (partition: string | undefined) => EphemeralHost;
    /** The machine's sockets, which hibernate the tunnels' sockets. */
    readonly #hibernation: HibernationHost;

    /** Host a region's relay in the machine's process. */
    constructor(
        options: Omit<RelayHostOptions, "tunnels"> & {
            /** Open each instance's memory, the root's under no partition. */
            readonly memory: (partition: string | undefined) => EphemeralHost;
            /** The machine's sockets, which hibernate the tunnels' sockets. */
            readonly hibernation: HibernationHost;
        },
    ) {
        // keep the memory, the sockets and the host calling its instances here
        this.#memory = options.memory;
        this.#hibernation = options.hibernation;
        this.host = new RelayHost({ ...options, tunnels: (request) => this.tunnels(request) });
    }

    /** Relay a request the machine does not serve itself: a name, or a tunnel's opening. */
    async fetch(request: Request): Promise<Response> {
        return this.host.names(await this.#instance(undefined), request);
    }

    /** Start the root, which follows the universe before any tunnel opens. */
    async start(): Promise<void> {
        await this.#instance(undefined);
    }

    /** The partitions started so far, the root's as absent. */
    get partitions(): readonly (string | undefined)[] {
        return [...this.#instances.keys()];
    }

    /** Call the root, or the tunnel a call's partition names, as the machine. */
    async tunnels(request: Request): Promise<Response> {
        return this.host.call(await this.#instance(RelayHost.partition(request)), request);
    }

    /** Stop the instances, take a step meanwhile, then start the same ones again, sockets kept. */
    async restart(meanwhile: () => Promise<void>): Promise<void> {
        // stop every instance, keeping the sockets the machine hibernates
        const stopped = [...this.#instances];
        this.#instances.clear();
        for (const [, started] of stopped) {
            await (await started).close();
        }

        // take the step, then start the same instances again
        await meanwhile();
        for (const [partition] of stopped) {
            await this.#instance(partition);
        }
    }

    /** Hand a hibernated socket's event to the tunnel that accepted the socket, by partition. */
    async receive(instanceId: string, request: Request): Promise<void> {
        const started = await this.#instance(instanceId);
        await this.#hibernation.instance(instanceId).receive(request, started.sockets);
    }

    /** Release every tunnel's sockets, as an instance stopping does, then close every instance. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const [partition, started] of this.#instances) {
            if (partition !== undefined) {
                this.#hibernation.release(partition);
            }
            await (await started).close();
        }
    }

    /** Start the root's or a tunnel's instance once. */
    #instance(partition: string | undefined): Promise<WorkloadInstance> {
        // take a started instance
        const existing = this.#instances.get(partition);
        if (existing !== undefined) {
            return existing;
        }

        // start it, a tunnel's sockets at the machine under its partition, the same after a restart
        const started = this.host.start(partition, {
            memory: this.#memory(partition),
            ...(partition === undefined
                ? {}
                : { hibernation: this.#hibernation.instance(partition) }),
        });
        this.#instances.set(partition, started);

        return started;
    }
}
