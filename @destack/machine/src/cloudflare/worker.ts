import { DistributionManifest } from "@destack/package";
import { WorkerdDistribution } from "@destack/package/workerd";
import { ServerBundle } from "./bundle.ts";
import { Once } from "@destack/package/manifest";
import { ProcessRuntime } from "@destack/stack/runtime";
import { traceSampling } from "@destack/stack/setting";
import { Directory, type SpaceAddress, UniverseOrigin } from "@destack/account/address";
import * as accountObject from "@destack/account/object";
import { AlarmClock, Snapshot } from "@destack/db";
import { type DurableObjectStorage, Jurisdiction, CONTENT_PATH } from "@destack/db/cloudflare";
import { Forwarded } from "@destack/environment";
import { WorkerdEnvironment } from "@destack/environment/workerd";
import { ClaimLookup, claimTable, InstallationMount, Egress, refusal } from "@destack/object";
import { Bearer } from "@destack/access/token";
import {
    ALARM_MILLISECONDS,
    BUILD_PATH,
    type DurableObjectWorkloadInstance,
} from "@destack/object/cloudflare";
import type { Alarm } from "@destack/db";
import { CloudMachine, CONNECTING_IP_HEADER, type MachineBindings } from "./cloud.ts";
import { type Identifier, present, schema } from "@destack/schema";
import { SettingValue } from "@destack/setting/object";
import { Route } from "@destack/stack/object";
import { TelemetryScope } from "@destack/telemetry";
import {
    type HibernatingSocket,
    FacetHost,
    type InstallationFacet,
    PartitionName,
    type PartitionState,
    WorkerdRuntime,
} from "./runtime/index.ts";
import { Machine } from "../machine/index.ts";
import machinePackage from "../package.ts";
import { type Space, SPACE_HEADER, SpaceRequest } from "../space/index.ts";
import { Attribution, TRACES_PATH } from "./attribution.ts";
import { TraceCount } from "../gateway/index.ts";
import { DurableObjectSpaceStorage } from "./storage.ts";
import { RateCard } from "./rates.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

export { DurableObjectDatabase } from "@destack/db/cloudflare/durable";
export { GraphIndexObject } from "@destack/stack/cloudflare";

/** The key of the space an object serves, which it resumes after an eviction. */
const SPACE_KEY = "space";

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

/** The space a partition's object serves, with its storage and its instances' runtime. */
interface ServedSpace {
    /** The space. */
    readonly space: Space;
    /** The space's storage in the object. */
    readonly host: DurableObjectSpaceStorage;
    /** The runtime running the space's instances in the object's facets. */
    readonly runtime: WorkerdRuntime;
}

/** A partition's object: the machine's directory, a space and its facets, or a partition. */
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

    /** Resume the space the object served before any event. */
    constructor(state: PartitionObjectState, bindings: MachineBindings) {
        // set the state, the storage, the bindings and the alarm clock
        this.#state = state;
        this.#storage = state.storage;
        this.#bindings = bindings;
        this.#alarms = new AlarmClock(state.storage, state.storage);
        this.#facets = new FacetHost(state, bindings, WorkerdEnvironment.system, this.#alarms);

        // follow the directory in the machine partition's object
        const objects = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);
        const machine = objects.idFromName(bindings.DESTACK_MACHINE_ID);
        this.#directory =
            state.id.toString() === String(machine)
                ? state.blockConcurrencyWhile(async () =>
                      (await this.#machine()).startDirectory(
                          state.storage,
                          this.#alarms.alarm(DIRECTORY_PARTY),
                      ),
                  )
                : undefined;

        // resume the space served before
        this.#served = state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get(SPACE_KEY);

            return stored === undefined ? undefined : this.#start(schema.string().parse(stored));
        });
    }

    /** Serve a request. */
    async fetch(request: Request): Promise<Response> {
        // answer the reads of the machine's directory replica
        if (this.#directory !== undefined) {
            return (await this.#directory).fetch(request);
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
            // serve the space
            else {
                return await served.space.fetch(request);
            }
        } catch (error) {
            return refusal(error, WorkerdEnvironment.system.telemetry);
        }
    }

    /** Hand an accepted WebSocket's message to its installation's workload. */
    webSocketMessage(socket: HibernatingSocket, message: ArrayBuffer | string): Promise<void> {
        return this.#facets.socket(socket).webSocketMessage(socket, message);
    }

    /** Hand an accepted WebSocket's close to its installation's workload. */
    webSocketClose(socket: HibernatingSocket): Promise<void> {
        return this.#facets.socket(socket).webSocketClose(socket);
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
        // run the directory's controllers
        const { clock } = WorkerdEnvironment.system;
        if (party === DIRECTORY_PARTY) {
            await (
                await present(this.#directory, "the machine's directory")
            ).alarm(clock.now() + ALARM_MILLISECONDS);
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
                MachineError.of("SPACE_STARTING", "no space runs here yet"),
                WorkerdEnvironment.system.telemetry,
            );
        }

        // count the posted work
        const { space } = served;
        for (const counted of schema.array(TraceCount).parse(await request.json())) {
            space.traffic.countTrace(space.id, counted);
        }

        return new Response(null, { status: 204 });
    }

    /** Start the space on the cloud machine over the object's storage. */
    async #start(spaceId: string): Promise<ServedSpace> {
        // open the space's storage in the object as the cloud machine
        const space = schema.identifier("space").parse(spaceId);
        const bindings = this.#bindings;
        const cloud = await this.#machine();
        const instruments = TelemetryScope.of(
            import.meta.destack.package,
            cloud.environment.telemetry,
        );
        const report = (error: unknown) => instruments.captureException(error, { isEscaped: true });
        const rates = RateCard.parse(JSON.parse(bindings.DESTACK_RATES));
        const host = await DurableObjectSpaceStorage.open({
            space,
            environment: cloud.environment,
            storage: this.#storage,
            files: bindings.FILES,
            filesName: bindings.DESTACK_FILES,
            domain: bindings.DESTACK_SPACE_DOMAIN,
            artifacts: bindings.ARTIFACTS,
            facet: (installation) => this.#facets.facet(installation),
            graphs: bindings.GRAPH,
            jurisdiction: bindings.DESTACK_JURISDICTION,
            gigabytes: rates.gigabytes,
            keyring: cloud.keyring,
            send: (request) => cloud.environment.network.fetch(request),
            mail: cloud.mail(),
            report,
        });

        // run the space's instances in the object's facets, rooted in the space's identity
        let served: Space | undefined;
        const directory = new Directory(Snapshot.live(cloud.replica));
        const { database } = host;
        const runtime = new WorkerdRuntime({
            instances: Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION),
            facet: (installation) => this.#facets.facet(installation),
            loader: bindings.LOADER,
            space,
            egress: EGRESS,
            sampling: (scope, installation) =>
                SettingValue.resolve(Snapshot.live(database), traceSampling, {
                    scope,
                    installation,
                }),
            url: async (scope, installation) => {
                const { url } = await directory.locate(scope, cloud.environment.clock.now());

                return InstallationMount.url(url, scope, installation);
            },
            identity: async () => {
                if (served === undefined) {
                    throw MachineError.of("SPACE_STARTING", `${space} is starting`);
                }

                return served.host.identity();
            },
            random: cloud.environment.random,
            builds: () => {
                if (served === undefined) {
                    throw MachineError.of("SPACE_STARTING", `${space} is starting`);
                }

                return served.builds;
            },
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
            enrollment: (view) => cloud.enrollment(view ?? Snapshot.live(cloud.replica)),
            spaces: host,
            cache: await host.cache(bindings.BUILDS, await cloud.releases(), shipped),
            isBuilding: false,
            isEnforcingEntitlements: true,
            // TODO #Incomplete: serve accounts' own hostnames once Cloudflare for SaaS provisions their certificates, after launch
            isServingHostnames: false,
            domain: bindings.DESTACK_SPACE_DOMAIN,
            runtimes: [runtime, new ProcessRuntime()],
            hibernation: undefined,
            alarm: this.#alarms.alarm(SPACE_PARTY),
            // run in process the servers the deploy bundled with the machine
            server: (manifest) => Promise.resolve(bundle.server(manifest)),
            linked: DistributionManifest.builtins(shipped),
            shipped: DistributionManifest.releases(shipped),
            registry: bindings.DESTACK_REGISTRY,
            skus: rates.skus,
            endpoint: `https://${bindings.DESTACK_HOST}`,
            version: machinePackage.version,
            authenticate: async (request, audience, partition) => {
                // verify the universe's or a space's token for the package
                const verified = await accountObject.Identity.authenticate(
                    directory.snapshot,
                    request,
                    {
                        audience,
                        universe: cloud.issuer,
                        now: cloud.environment.clock.now(),
                        environment: cloud.environment,
                    },
                );
                if (verified === undefined) {
                    return undefined;
                }

                // require a call into the space bound to it, or to the account the space is the home of
                if (partition === space) {
                    await (await machine.space(space)).host.requirePartition(verified);
                }
                // refuse a token bound to another partition
                else if (partition !== undefined) {
                    throw MachineError.of(
                        "FOREIGN_PARTITION_TOKEN",
                        "the universe token is bound to another partition",
                    );
                }

                return verified;
            },
            authorizeSpace: async ({ access }) => {
                // refuse machine procedures, which only a computer's owner calls, and pass every other call
                if (access.authentication === "machine") {
                    throw MachineError.of(
                        "OWNER_MISSING",
                        "no owner can call a cloud machine's machine procedures",
                    );
                }
            },
            report,
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

    /** Answer a request in the object of its space in the residency's jurisdiction. */
    async fetch(given: Request, bindings: MachineBindings): Promise<Response> {
        // take a public request's client from Cloudflare, and the head a service binding hands on as it is
        const client = given.headers.get(CONNECTING_IP_HEADER);
        const request = client === null ? given : Forwarded.replace(given, { for: client });

        // hand the universe its issuer and package mounts at its origin, which it serves behind the machines
        if (UniverseOrigin.of(request.url, bindings.DESTACK_ISSUER) !== undefined) {
            return bindings.UNIVERSE.fetch(request);
        }

        // find the spaces served here alone, by a request to the machine's own host or a name
        const isOwn = new URL(request.url).host === bindings.DESTACK_HOST;
        const served = async () => {
            const { replica, id } = await this.#machine(bindings);

            return SpaceRequest.served(new Directory(Snapshot.live(replica)), id);
        };
        const origins = {
            domain: bindings.DESTACK_SPACE_DOMAIN,
            find: async (address: SpaceAddress) => (await served()).find(address),
            serves: async (space: Identifier<"space">) => (await served()).serves(space),
        };

        // find the space of a name: its installation origin's or S3 origin's, else its route's
        const routed = isOwn
            ? await SpaceRequest.of(request, origins)
            : await SpaceRequest.named(request, {
                  ...origins,
                  route: async (host, path) => {
                      // match the route's hostname claims in the directory replica
                      const { replica } = await this.#machine(bindings);
                      const claims = ClaimLookup.of(Snapshot.live(replica).read(claimTable));

                      return Route.locate(claims, host, path);
                  },
              });

        // refuse a request outside every space, or to the path the tail posts traced work at
        if (routed === undefined || new URL(routed.request.url).pathname === TRACES_PATH) {
            return new Response(null, { status: 404 });
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
