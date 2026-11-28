import { OtlpExporter } from "@destack/telemetry/otlp";
import { TokenVerifier, Bearer } from "@destack/access/token";
import {
    CLOUD_RUNTIMES,
    CloudMachine,
    CONNECTING_IP_HEADER,
    type MachineBindings,
} from "./cloud.ts";
import { DistributionManifest } from "@destack/package";
import { WorkerdDistribution } from "@destack/package/workerd";
import { ServerBundle } from "./bundle.ts";
import { ProcessRuntime } from "../runtime/process.ts";
import { traceSampling } from "@destack/stack/setting";
import type { SpaceAddress } from "@destack/account/address";
import { Directory } from "@destack/account/client";
import * as accountObject from "@destack/account/object";
import { AlarmClock, type Alarm } from "@destack/db/database";
import {
    type DurableObjectStorage,
    DurableObjectDatabaseHost,
    Jurisdiction,
    CONTENT_PATH,
} from "@destack/db/cloudflare";
import { Forwarded, Once, WebSocketUpgrade } from "@destack/environment";
import { WorkerdEnvironment } from "@destack/environment/workerd";
import { ClaimLookup, claimTable, InstallationMount, Egress, refusal } from "@destack/object";
import {
    ALARM_MILLISECONDS,
    BUILD_PATH,
    DurableObjectHibernation,
    type DurableObjectWorkloadInstance,
    type Upgrade,
} from "@destack/object/cloudflare";
import {
    type EphemeralHost,
    HibernationHost,
    SOCKET_HEADER,
    SocketRefusal,
    type WorkloadInstance,
} from "@destack/object/workload";
import { RelayHost } from "@destack/relay/server";
import { type Identifier, present, schema } from "@destack/schema";
import { SettingValue } from "@destack/setting/object";
import { Route } from "@destack/stack/object";
import {
    type HibernatingSocket,
    FacetHost,
    type InstallationFacet,
    PartitionName,
    type PartitionState,
    WorkerdRuntime,
} from "./runtime/index.ts";
import { Machine, OLDEST_DESTACK } from "../machine/index.ts";
import machinePackage from "../package.ts";
import {
    type Space,
    SPACE_HEADER,
    SPACE_RETRY_MILLISECONDS,
    SpaceRequest,
} from "../space/index.ts";
import { Attribution, TRACES_PATH } from "./attribution.ts";
import { TraceCount } from "../gateway/index.ts";
import { DurableObjectSpaceStorage } from "./storage.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

export { DurableObjectDatabase } from "@destack/db/cloudflare/durable";
export { GraphIndexObject } from "./graph.ts";

/** The key of the space an object serves, which it resumes after an eviction. */
const SPACE_KEY = "space";

/** The path the machine's directory starts a placed space at, as a space starts on a request. */
const START_PATH = "/.destack/space/start";

/** The party of the object's alarm that runs the machine's directory controllers. */
const DIRECTORY_PARTY = "directory";

/** The party of the object's alarm that runs the space's controllers. */
const SPACE_PARTY = "space";

/** The egress a space's instances call out through, below the space's object. */
const EGRESS = `https://space${Egress.path}`;

/** The storage of a space's object: its databases, the space it serves, and its alarm. */
export type PartitionStorage = DurableObjectStorage &
    Alarm & {
        /** Read a stored value. */
        readonly get: (key: string) => Promise<unknown>;
        /** Store a value. */
        readonly put: (key: string, value: unknown) => Promise<void>;
    };

/** The state a partition's object runs over. */
type PartitionObjectState = PartitionState & {
    /** The object's identifier, the same for each name, with the name it was addressed by. */
    readonly id: { readonly toString: () => string; readonly name?: string };
    /** The object's storage. */
    readonly storage: PartitionStorage;
    /** Run a closure before the object takes any event. */
    readonly blockConcurrencyWhile: <Value>(closure: () => Promise<Value>) => Promise<Value>;
};

/** The tag of the space store's WebSockets, beside the installations' tags their facets keep. */
const STORE_SOCKET_TAG = "store";

/** The tag of a relay tunnel's WebSockets, in its own object. */
const TUNNEL_SOCKET_TAG = "tunnel";

/** The host relay instances call each other at, which stays inside the machine's objects. */
const TUNNELS_HOST = "tunnels";

/** The space a partition's object serves, with its storage and its instances' runtime. */
interface ServedSpace {
    /** The space. */
    readonly space: Space;
    /** The space's storage in the object. */
    readonly host: DurableObjectSpaceStorage;
    /** The runtime running the space's instances in the object's facets. */
    readonly runtime: WorkerdRuntime;
}

/** A relay instance a relay object runs: the root, or a tunnel with its sockets. */
interface RelayObject {
    /** The host the instance started from. */
    readonly host: RelayHost;
    /** The instance. */
    readonly instance: WorkloadInstance;
    /** The host of a tunnel's sockets, which the object hibernates. */
    readonly sockets: HibernationHost;
}

/** A partition's object: the directory, a relay instance, a space and facets, or a partition. */
export class PartitionObject {
    /** The object's state. */
    readonly #state: PartitionObjectState;
    /** The object's storage. */
    readonly #storage: PartitionStorage;
    /** The Worker's bindings. */
    readonly #bindings: MachineBindings;
    /** The object's one alarm, shared by the directory, the space and each installation's facet. */
    readonly #alarms: AlarmClock;
    /** The cloud machine the object runs as, once read. */
    readonly #cloud = new Once<CloudMachine>();
    /** The machine's directory replica, in the machine partition's object alone. */
    readonly #directory: Promise<DurableObjectWorkloadInstance> | undefined;
    /** The space the object serves, once its partition started. */
    #served: Promise<ServedSpace | undefined>;
    /** The facets of the installations the object hosts. */
    readonly #facets: FacetHost;
    /** The space store's WebSockets, kept for its machine's host across hibernation. */
    readonly #storeHibernation: DurableObjectHibernation;
    /** The relay instance the object runs, in a relay object alone. */
    readonly #relay: Promise<RelayObject> | undefined;
    /** A relay tunnel's WebSockets, kept for its sockets' host across hibernation. */
    readonly #tunnelHibernation: DurableObjectHibernation;

    /** Resume the space the object served before any event. */
    constructor(state: PartitionObjectState, bindings: MachineBindings) {
        // set the state, the storage, the bindings and the alarm clock
        this.#state = state;
        this.#storage = state.storage;
        this.#bindings = bindings;
        this.#alarms = new AlarmClock(state.storage, state.storage);
        this.#facets = new FacetHost(state, bindings, WorkerdEnvironment.system, this.#alarms);
        this.#storeHibernation = new DurableObjectHibernation(state, STORE_SOCKET_TAG);
        this.#tunnelHibernation = new DurableObjectHibernation(state, TUNNEL_SOCKET_TAG);

        // run the relay's root, or a tunnel, in a relay object
        const relayed = CloudMachine.relayPartition(state.id.name);
        this.#relay =
            relayed === undefined
                ? undefined
                : state.blockConcurrencyWhile(() => this.#startRelay(relayed.partition));

        // follow the directory in the machine partition's object
        const objects = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);
        const machine = objects.idFromName(bindings.DESTACK_MACHINE_ID);
        this.#directory =
            state.id.toString() === String(machine)
                ? state.blockConcurrencyWhile(async () =>
                      (await this.#machine()).startDirectory(
                          state.storage,
                          this.#alarms.alarm(DIRECTORY_PARTY),
                          (space) => this.#startSpace(space),
                      ),
                  )
                : undefined;

        // read the space served before under the input gate, then resume it as a request starts it
        const stored = state.blockConcurrencyWhile(() => state.storage.get(SPACE_KEY));
        this.#served = stored.then((value) =>
            value === undefined ? undefined : this.#start(schema.string().parse(value)),
        );
    }

    /** Serve a request. */
    async fetch(request: Request): Promise<Response> {
        // answer the reads of the machine's directory replica
        if (this.#directory !== undefined) {
            return (await this.#directory).fetch(request);
        }
        // answer a relay instance's calls and names
        else if (this.#relay !== undefined) {
            return this.#relayed(await this.#relay, request);
        }

        // add the work the tail traced to the space's traffic
        const url = new URL(request.url);
        if (url.pathname === TRACES_PATH) {
            return this.#count(request);
        }

        // start the space of the header once, forgetting a failed start so the next request tries again
        const spaceId = request.headers.get(SPACE_HEADER);
        const starting = this.#served.then(async (served) => {
            if (served !== undefined || spaceId === null) {
                return served;
            }
            await this.#storage.put(SPACE_KEY, spaceId);

            return this.#start(spaceId);
        });
        this.#served = starting.catch(() => undefined);
        try {
            // hand a workload's call to its own facet, and serve a partition's facet in an object serving no space
            const served = await starting;
            if (url.pathname === START_PATH) {
                return new Response(null, { status: 204 });
            }
            const routed = this.#facets.route(request);
            if (routed !== undefined) {
                return await routed;
            } else if (served === undefined) {
                return await this.#partition().fetch(request);
            }
            Attribution.publish({ scope: served.space.id });

            // serve while the machine's account enables it, as its directory copy says, else refuse with why
            const { machine } = served.space;
            await machine.updateHealth();
            const refused = machine.health.refusal();
            if (refused !== undefined) {
                throw refused;
            }

            // read every module of an instance's build and its dependencies, their imports resolved
            if (url.pathname === BUILD_PATH) {
                return Response.json(await served.runtime.modules(Bearer.require(request.headers)));
            }
            // serve a page of a database replica an installation's facet copies
            else if (url.pathname.startsWith(`${CONTENT_PATH}/`)) {
                return await served.host.content(request);
            }
            // send an npm client on, else serve the space, keeping the socket of an upgrade its store accepts
            else {
                const npm = await machine.npm(request);
                if (npm !== undefined) {
                    return npm;
                }
                const answer = await served.space.fetch(request);
                const isAccepted =
                    WebSocketUpgrade.of(request) !== undefined && answer.headers.has(SOCKET_HEADER);

                return isAccepted
                    ? this.#storeHibernation.accept(
                          present(served.space.machine.hibernation, "the machine's sockets"),
                          answer,
                      )
                    : answer;
            }
        } catch (error) {
            return refusal(error, WorkerdEnvironment.system.telemetry);
        }
    }

    /** Hand an accepted WebSocket's message to the space's store, else to its installation. */
    async webSocketMessage(
        socket: HibernatingSocket,
        message: ArrayBuffer | string,
    ): Promise<void> {
        // hand a tunnel's message to its sockets' host
        if (this.#tunnelHibernation.has(socket)) {
            const { sockets } = await present(this.#relay, "the tunnel of the socket");
            this.#tunnelHibernation.message(sockets, socket, message);
        }
        // hand the store's message to its machine's host, which delivers each in order
        else if (this.#storeHibernation.has(socket)) {
            this.#storeHibernation.message(await this.#storeHost(), socket, message);
        }
        // hand an installation's to its facet
        else {
            await this.#facets.socket(socket).webSocketMessage(socket, message);
        }
    }

    /** Hand an accepted WebSocket's close to the space's store, else to its installation. */
    async webSocketClose(socket: HibernatingSocket, code: number, reason: string): Promise<void> {
        // hand a tunnel's close to its sockets' host
        if (this.#tunnelHibernation.has(socket)) {
            const { sockets } = await present(this.#relay, "the tunnel of the socket");
            this.#tunnelHibernation.close(sockets, socket, code, reason);
        }
        // hand the store's close to its machine's host
        else if (this.#storeHibernation.has(socket)) {
            this.#storeHibernation.close(await this.#storeHost(), socket, code, reason);
        }
        // hand an installation's to its facet
        else {
            await this.#facets.socket(socket).webSocketClose(socket, code, reason);
        }
    }

    /** Answer a relay instance's call as the machine, else hand a name to the root's names. */
    async #relayed(running: RelayObject, request: Request): Promise<Response> {
        // hand a name the Worker relays to the root's names
        const { host, instance, sockets } = running;
        if (new URL(request.url).host !== TUNNELS_HOST) {
            return host.names(instance, request);
        }

        // call the instance, keeping the socket of an upgrade a tunnel accepts in this object
        const answer = await host.call(instance, request);
        const isAccepted =
            WebSocketUpgrade.of(request) !== undefined && answer.headers.has(SOCKET_HEADER);

        return isAccepted ? this.#tunnelHibernation.accept(sockets, answer) : answer;
    }

    /** Start the relay's root or a tunnel over the object's storage, its sockets kept here. */
    async #startRelay(partition: string | undefined): Promise<RelayObject> {
        // keep the instance's memory in the object's storage, which outlives hibernation
        const cloud = await this.#machine();
        const host = await cloud.relay();
        const storage = new DurableObjectDatabaseHost(this.#storage, cloud.environment);
        const memory: EphemeralHost = { open: async (tables) => storage.connect(tables) };

        // hibernate a tunnel's sockets under its partition, delivering their events to the instance
        const sockets: HibernationHost = new HibernationHost(async (instanceId, path, request) => {
            const { instance } = await present(this.#relay, "the relay instance of the socket");
            await sockets
                .instance(instanceId)
                .receive(new Request(`http://relay${path}`, request), instance.sockets);

            return new Response(null, { status: 204 });
        }, cloud.environment.telemetry);
        const instance = await host.start(partition, {
            memory,
            ...(partition === undefined ? {} : { hibernation: sockets.instance(partition) }),
        });

        return { host, instance, sockets };
    }

    /** Read the host of the served space's machine, which keeps the store's sockets. */
    async #storeHost(): Promise<HibernationHost> {
        const served = present(await this.#served, "the space whose store accepted the socket");

        return present(served.space.machine.hibernation, "the machine's sockets");
    }

    /** Wake the directory, the space or each facet whose alarm is due, for at most ten minutes. */
    async alarm(): Promise<void> {
        await this.#alarms.fire(WorkerdEnvironment.system.clock.now(), async (parties) => {
            for (const party of parties) {
                await this.#wake(party);
            }
        });
    }

    /** Wake one party of the object's alarm: the directory, the space or a facet. */
    async #wake(party: string): Promise<void> {
        // run the directory's controllers for a whole alarm window, then ring again at once
        const { clock } = WorkerdEnvironment.system;
        if (party === DIRECTORY_PARTY) {
            const directory = await present(this.#directory, "the machine's directory");
            const deadline = clock.now() + ALARM_MILLISECONDS;
            await Promise.all([directory.alarm(deadline), clock.sleep(ALARM_MILLISECONDS)]);
            await this.#alarms.alarm(DIRECTORY_PARTY).setAlarm(clock.now());
        }
        // run the space's controllers while the machine's account enables it
        else if (party === SPACE_PARTY) {
            const served = present(await this.#served, "the object's space");
            Attribution.publish({ scope: served.space.id });
            const { machine } = served.space;
            await machine.updateHealth();
            if (machine.health.status === "up") {
                await served.space.instance.alarm(clock.now() + ALARM_MILLISECONDS);
            }
        }
        // run an installation's workload
        else {
            await this.#facets.facet(party).alarm();
        }
    }

    /** Read the facet of the installation a partition's object serves, named by the object. */
    #partition(): InstallationFacet {
        const { installation } = PartitionName.read(
            present(this.#state.id.name, "the object's name"),
        );

        return this.#facets.facet(present(installation, "the partition's installation"));
    }

    /** Read the cloud machine the object runs as, deriving it again after a failed derivation. */
    #machine(): Promise<CloudMachine> {
        return this.#cloud.get(() => CloudMachine.read(this.#bindings, WorkerdEnvironment.system));
    }

    /** Add the work a space's traces measured, as the machine's tail posts it, to its traffic. */
    async #count(request: Request): Promise<Response> {
        // refuse an object serving no space
        const served = await this.#served;
        if (served === undefined) {
            return refusal(
                MachineError.of(
                    "SPACE_UNAVAILABLE",
                    "space is unavailable: no space runs here yet",
                    { data: {} },
                ),
                WorkerdEnvironment.system.telemetry,
            );
        }

        // count the posted work
        const { space } = served;
        for (const counted of schema.array(TraceCount).parse(await request.json())) {
            space.machine.traffic.countTrace(space.id, counted);
        }

        return new Response(null, { status: 204 });
    }

    /** Start a placed space in its object, refusing a start the object fails. */
    async #startSpace(space: Identifier<"space">): Promise<void> {
        // ask the space's object at the start path, as the machine's own host
        const bindings = this.#bindings;
        const namespace = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);
        const object = namespace.get(namespace.idFromName(PartitionName.of({ space })));
        const request = new Request(`${bindings.DESTACK_ORIGIN}${START_PATH}`, {
            method: "POST",
            headers: { [SPACE_HEADER]: space },
        });
        const response = await object.fetch(request);

        // refuse a failed start with what the object answered
        if (!response.ok) {
            throw MachineError.of(
                "SPACE_UNAVAILABLE",
                `'${space}' is unavailable: its start answered ${response.status}: '${await response.text()}'`,
                { data: {} },
            );
        }
    }

    /** Start the space on the cloud machine over the object's storage. */
    async #start(spaceId: string): Promise<ServedSpace> {
        // open the space's storage in the object as the cloud machine
        const space = schema.identifier("space").parse(spaceId);
        const bindings = this.#bindings;
        const cloud = await this.#machine();
        const host = await DurableObjectSpaceStorage.open({
            space,
            environment: cloud.environment,
            storage: this.#storage,
            bucket: bindings.BUCKET,
            bucketName: bindings.DESTACK_BUCKET,
            domain: bindings.DESTACK_SPACE_DOMAIN,
            artifacts: bindings.ARTIFACTS,
            facet: (installation) => this.#facets.facet(installation),
            graphs: bindings.GRAPH,
            jurisdiction: bindings.DESTACK_JURISDICTION,
            keyring: cloud.keyring,
            send: (request) => cloud.environment.network.fetch(request),
            mail: cloud.mail(),
        });

        // run the space's instances in the object's facets, rooted in the space's identity
        let served: Space | undefined;
        const directory = new Directory(cloud.replica.snapshot());
        const verifier = new TokenVerifier({
            issuers: (issuer) =>
                accountObject.Identity.trust(directory.snapshot, issuer, cloud.issuer),
            environment: cloud.environment,
        });
        const { database } = host;
        const runtime = new WorkerdRuntime({
            instances: Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION),
            facet: (installation) => this.#facets.facet(installation),
            loader: bindings.LOADER,
            space,
            egress: EGRESS,
            // sample each installation as its settings set, batching at OpenTelemetry's default
            workloadTelemetry: async (scope, installation) => ({
                sampling: await SettingValue.resolve(database.snapshot(), traceSampling, {
                    scope,
                    installation,
                }),
                delay: OtlpExporter.DEFAULT_DELAY,
            }),
            url: async (scope, installation) => {
                const { url } = await directory.locate(scope, cloud.environment.clock.now());

                return InstallationMount.url(url, scope, installation);
            },
            identity: async () => {
                if (served === undefined) {
                    throw MachineError.of(
                        "SPACE_UNAVAILABLE",
                        `'${space}' is unavailable: it is starting`,
                        { data: { retryAfter: SPACE_RETRY_MILLISECONDS } },
                    );
                }

                return served.identity();
            },
            random: cloud.environment.random,
            served: () => {
                if (served === undefined) {
                    throw MachineError.of(
                        "SPACE_UNAVAILABLE",
                        `'${space}' is unavailable: it is starting`,
                        { data: { retryAfter: SPACE_RETRY_MILLISECONDS } },
                    );
                }

                return served;
            },
            telemetry: cloud.environment.telemetry,
        });

        // serve the space on the one machine, enrolled by the universe's declaration of it, started now
        const shipped = await WorkerdDistribution.read();
        const bundle = await ServerBundle.read(shipped);
        const startedAt = cloud.environment.clock.now();
        const machine = new Machine({
            id: cloud.id,
            environment: cloud.environment,
            identity: cloud.identity,
            keyring: cloud.keyring,
            database: cloud.replica,
            describe: async () => ({ osVersion: null, startedAt, update: null, host: null }),
            enrollment: (view) => cloud.enrollment(view ?? cloud.replica.snapshot()),
            storage: host,
            builds: await host.builds(await cloud.releases()),
            targets: CLOUD_RUNTIMES,
            isRunner: false,
            isEnforcingEntitlements: true,
            // TODO #Incomplete: serve accounts' own hostnames once Cloudflare for SaaS provisions their certificates, after launch
            isServingHostnames: false,
            domain: bindings.DESTACK_SPACE_DOMAIN,
            runtimes: [runtime, new ProcessRuntime()],
            // keep the space store's sockets at the machine's host, installations' facets keeping their own
            instances: (instanceId) => {
                throw MachineError.of(
                    "SOCKET_HANDLER_NOT_FOUND",
                    `socket handler of '${instanceId}' not found: an installation's facet keeps its sockets`,
                );
            },
            alarm: this.#alarms.alarm(SPACE_PARTY),
            // run in process the servers the deploy bundled with the machine
            server: (image) => Promise.resolve(bundle.server(image)),
            views: (image) => Promise.resolve(bundle.views(image)),
            linked: DistributionManifest.builtins(shipped, "workerd"),
            shipped: DistributionManifest.releases(shipped, "workerd"),
            endpoint: bindings.DESTACK_ORIGIN,
            destack: { oldest: OLDEST_DESTACK, newest: machinePackage.version },
            authenticate: async (request, audience, partition) => {
                // verify the universe's or a space's token for the package
                const verified = await accountObject.Identity.authenticate(
                    directory.snapshot,
                    request,
                    verifier,
                    { audience, now: cloud.environment.clock.now() },
                );
                if (verified === undefined) {
                    return undefined;
                }

                // require a call into the space bound to it, or to the account the space is the home of
                if (partition === space) {
                    await (await machine.space(space)).requirePartition(verified);
                }
                // refuse a token bound to another partition
                else if (partition !== undefined) {
                    throw MachineError.of(
                        "PARTITION_TOKEN_DENIED",
                        "universe token is forbidden: it is bound to another partition",
                    );
                }

                return verified;
            },
            authorizeSpace: async ({ access }) => {
                // refuse machine procedures, which only a computer's owner calls, and pass every other call
                if (access.authentication === "machine") {
                    throw MachineError.of(
                        "OWNER_DENIED",
                        "machine procedure is forbidden: no owner can call a cloud machine's machine procedures",
                    );
                }
            },
        });
        // read the machine's health from its directory copy, then start the space, which refuses unless it serves
        await machine.updateHealth();
        served = await machine.space(space);

        return { space: served, host, runtime };
    }
}

/** The machine Worker's handler in an isolate, handing each request to the object of its space. */
export class MachineWorker {
    /** The cloud machine the isolate resolves origins' spaces as, once derived. */
    readonly #cloud = new Once<CloudMachine>();

    /** Serve a request in the object of its space, stating a refused upgrade on a socket. */
    async fetch(given: Request, bindings: MachineBindings): Promise<Response> {
        // serve the request, passing on anything but a refused upgrade
        const response = await this.#serve(given, bindings);
        const refused = await SocketRefusal.of(given, response);
        if (refused === undefined) {
            return response;
        }

        // open a socket in the isolate to state the refusal, which a browser cannot read from the response
        const pair = new WebSocketPair();
        pair[1].accept();
        const end = pair[1];
        await refused.send({
            send: async (message) => end.send(message),
            close: (code, reason) => end.close(code, reason),
        });
        const upgrade: Upgrade = { status: 101, webSocket: pair[0] };

        return new Response(null, upgrade);
    }

    /** Serve a request in the object of its space in the region's jurisdiction. */
    async #serve(given: Request, bindings: MachineBindings): Promise<Response> {
        // take a public request's client from Cloudflare, and the head a service binding hands on as it is
        const client = given.headers.get(CONNECTING_IP_HEADER);
        const request = client === null ? given : Forwarded.replace(given, { for: client });

        // hand the universe its issuer and package mounts at its origin, which it serves behind the machines
        if (CloudMachine.universe(bindings).serves(request.url) !== undefined) {
            return bindings.UNIVERSE.fetch(request);
        }

        // find the spaces served here alone, by a request to the machine's own host or a name
        const isOwn = new URL(request.url).host === new URL(bindings.DESTACK_ORIGIN).host;
        const served = async () => {
            const { replica, id, environment } = await this.#machine(bindings);

            return SpaceRequest.served(new Directory(replica.snapshot()), id, environment.clock);
        };
        const origins = {
            domain: bindings.DESTACK_SPACE_DOMAIN,
            find: async (address: SpaceAddress) => (await served()).find(address),
            serves: async (space: Identifier<"space">) => (await served()).serves(space),
            locate: async (space: Identifier<"space">) => (await served()).locate(space),
        };

        // find the space of a name, refusing one not placed yet
        const routed = isOwn
            ? await SpaceRequest.of(request, origins)
            : await SpaceRequest.host(request, {
                  ...origins,
                  route: async (host, path, serves) => {
                      // match the route's hostname claims in the directory replica
                      const { replica } = await this.#machine(bindings);
                      const claims = ClaimLookup.of(replica.snapshot().read(claimTable));

                      return Route.locate(claims, host, path, serves);
                  },
              }).catch((error: unknown) => refusal(error, WorkerdEnvironment.system.telemetry));
        if (routed instanceof Response) {
            return routed;
        }

        // relay a request for no space here to the region's relay, a name of a machine or of a space elsewhere
        if (routed === undefined) {
            return (await this.#machine(bindings)).relayObject(undefined).fetch(request);
        }
        // refuse the path the tail posts traced work at
        else if (new URL(routed.request.url).pathname === TRACES_PATH) {
            return new Response(null, { status: 404 });
        }

        // move a request at a former name, or for a space placed elsewhere
        if (routed.moved !== undefined) {
            return SpaceRequest.move(routed.moved);
        }

        // attribute the invocation to the space, and forward the request to its partition's object
        Attribution.publish({ scope: routed.space });
        const namespace = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);
        const name = PartitionName.of({ space: routed.space });

        return namespace.get(namespace.idFromName(name)).fetch(routed.request);
    }

    /** Read the isolate's cloud machine, deriving it again after a failed derivation. */
    #machine(bindings: MachineBindings): Promise<CloudMachine> {
        return this.#cloud.get(() => CloudMachine.read(bindings, WorkerdEnvironment.system));
    }
}

/** The machine Worker's handler, one per isolate. */
export default new MachineWorker();
