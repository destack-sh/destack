import type { TunnelStatus } from "@destack/tunnel";
import { BunDistribution, Program } from "@destack/package/bun";
import { PackageMount, refusal, tablesOf } from "@destack/object";
import objectPackage from "@destack/object/package";
import { checkout, preview } from "./checkout/index.ts";
import { defineDatabase } from "@destack/db";
import { BUILD_STORE_TABLES } from "@destack/build/store";
import { build } from "@destack/stack/object";
import { join } from "node:path";
import { inspect } from "node:util";
import { Distribution, DISTRIBUTION_DIRECTORY } from "@destack/update";
import { Toolchain } from "@destack/check/toolchain";
import { distribution } from "@destack/update/object";
import { MachineIdentity } from "@destack/account/client";
import { login, type NativeClient } from "@destack/account/native";
import {
    call,
    Egress,
    InstallationMount,
    type PackageHandle,
    SpaceMount,
    CallKey,
} from "@destack/object";
import type { Server } from "@destack/object/server";
import { LocalBlobStorage, LocalBucket } from "@destack/bucket/bun";
import { CompilerPool, LocalBuildStore } from "@destack/build";
import { BuildLayer } from "@destack/build/store";
import { BlobStore, type DatabaseConnection, Snapshot, TABLE } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import { DatabasePersonalKeyring, type EventKind, EventStore } from "@destack/event";
import type { Registry } from "@destack/stack/server";
import { type Deriver, type Keychain, type Keyring, LocalKeyring } from "@destack/identity";
import { BunKeychain, type KeychainStore } from "@destack/identity/bun";
import { mailConnector } from "@destack/mail/transport";
import { PrintTransport } from "@destack/mail/print";
import type { ObjectType } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { traceSampling } from "@destack/stack/setting";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, present, schema } from "@destack/schema";
import { DistributionManifest } from "@destack/package";
import { serveProcess } from "@destack/object/bun";
import { HibernationHost, WorkloadInstance } from "@destack/object/workload";
import { deviceSetting, SettingValue } from "@destack/setting/object";
import { type BunIsolation, BunRuntime } from "../bun/index.ts";
import { ProcessRuntime, type Runtime } from "@destack/stack/runtime";
import { createClient, type ClientOf } from "@destack/object/client";
import { BuildCache } from "@destack/stack/server";
import { SeverityNumber, TelemetryScope } from "@destack/telemetry";
import { startTelemetry } from "@destack/telemetry/bun";
import { OTLP_ORIGIN_PATH, OtlpExporter } from "@destack/telemetry/otlp";
import stackPackage from "@destack/stack/package";
import type { Telemetry } from "@destack/telemetry/sdk";
import type { DistributionHost } from "@destack/update/server";
import { DRAIN_TIMEOUT_MILLISECONDS, Machine, type MachineOptions } from "../machine/index.ts";
import { Authenticator } from "./device/authentication/index.ts";
import { Device } from "./device/device.ts";
import { MachineError } from "../error/index.ts";
import { LOOPBACK_DOMAIN } from "@destack/account/address";
import { ComputerServer } from "./server/index.ts";
import { LocalSpaceStorage } from "./storage.ts";
import { computer, type MachineEnrollment, signIn, visit } from "./device/object/index.ts";
import { MachineProcess, LOCK_FILE, StageHome } from "./process/index.ts";
import { BuildRunner } from "./build.ts";
import { computerServer, machineWorkload, deviceWorkload } from "./server/workload.ts";
import { Environment, HostAddress, LineExporter } from "@destack/environment";
import { DOMAINS, MachineAddress } from "@destack/account/address";
import { PartitionRequest } from "../space/index.ts";

/** The origin the machine's own exports are addressed below until its owner's home is located. */
const OWN_TELEMETRY_ORIGIN = "http://machine.local";

/** The event name of a line an instance writes to standard error. */
const OUTPUT_EVENT = "runtime.output";

/** The nanoseconds of a millisecond, as OTLP times records. */
const NANOSECONDS_PER_MILLISECOND = 1_000_000n;

/** The fixed port the machine listens on, registered below the ephemeral ports (RFC 6335 6). */
export const MACHINE_PORT = 7470;

/** The package identity of the machine's authentication and telemetry. */
const MACHINE_PACKAGE = import.meta.destack.package;

/** The directory below the machine's with its keys, where it stores them in files. */
const KEYCHAIN_DIRECTORY = "keychain";

/** The file of the machine's own database, within its directory. */
const MACHINE_FILE = "machine.db";

/** The identifier schema of machines, whose partitions answer remote callers. */
const MACHINE_IDENTIFIER = schema.identifier("machine");

/** The keychain service of machine keys, by machine. */
export const MACHINE_KEYS = "app.destack.machine";

/** The keychain service of a machine's root keys, by machine. */
export const ROOT_KEYS = "app.destack.root";

/** The runtimes a computer runs builds on: Bun for its instances, browsers for its views. */
export const COMPUTER_RUNTIMES = ["bun", "browser"] as const;

/** What a machine runs with beside its state directory, which tests replace. */
export interface ComputerOptions {
    /** The directory with the machine's state, by its real path. */
    readonly directory: string;
    /** The host the machine runs on: its clock, network, files, processes, platform and server. */
    readonly environment: Environment<"files" | "process" | "host" | "server">;
    /** The origin of the universe people sign in at. */
    readonly universe: string;
    /** The port the machine's listener answers at. */
    readonly port: number;
    /** The origin peers call the machine at, its loopback listener's by default. */
    readonly endpoint?: string;
    /** The relay routing this machine's names, absent for no tunnel. */
    readonly relay?:
        | {
              /** At a relay's tunnel URL. */
              readonly kind: "url";
              /** The tunnel URL. */
              readonly url: string;
          }
        | {
              /** At the relay of the residency its account stores data in. */
              readonly kind: "residency";
          };
    /** The local object types the device stores beside its logins, settings and visits, by key. */
    readonly localObjects: Readonly<Record<string, ObjectType>>;
    /** The releases the computer ships: its servers, builtins and the platform's stacks. */
    readonly shipped: DistributionManifest;
    /** The shipped releases' builds with what they run, read in place. */
    readonly layer: BuildLayer;
    /** Open the keychain of a service's secrets. */
    readonly keychain: (service: string) => Keychain;
    /** The machine to run, the stored one or a new one by default. */
    readonly machineId?: Identifier<"machine">;
    /** The enrollment the universe declares for a cloud machine. */
    readonly enrollment?: MachineEnrollment;
    /** The running process the computer's record describes. */
    readonly process: {
        readonly version: string;
        readonly pid: number;
        readonly startedAt: number;
    };
    /** How the machine isolates its instances: in sandboxed processes, or in this process. */
    readonly isolation: BunIsolation;
    /** The installation residency homes serve the registry protocol at, such as forge. */
    readonly registry: string;
    /** Run the machine's instances that call addresses through an egress. */
    readonly runtime?: (
        egress: (scope: Identifier<"space">) => string,
        identity: (scope: Identifier<"space">) => Promise<Deriver>,
    ) => Runtime & { readonly close: () => Promise<void> };
    /** The forges' registry, by default the directory's forges called as the machine. */
    readonly releases?: Registry;
    /** The installed distribution the machine stages and activates. */
    readonly distribution?: DistributionHost;
    /** Read the toolchain the compilers run, once they first need it. */
    readonly toolchain: () => Promise<Toolchain>;
    /** Report a failure no request answers, by the event it ended. */
    readonly report: (event: string, error: unknown) => void;
    /** Stop the machine at a client's request. */
    readonly shutdown: () => void;
}

/** The Destack process on a computer: its machine, native device, view origins and gateway. */
export class Computer implements AsyncDisposable {
    /** The database of the machine's own partition on a computer. */
    static readonly database = defineDatabase({
        name: "machine",
        ...tablesOf({
            packages: [objectPackage, { objects: { checkout, preview }, tables: [] }],
            follows: [],
            // index the machine's build store beside its objects
            tables: BUILD_STORE_TABLES,
        }),
        // replicate the universe's directory and this machine's own work
        replicated: [build.table, ...Machine.replicated],
    });

    /** The machine serving its spaces while enrolled. */
    readonly machine: Machine;
    /** This computer's device, which this machine created, with its people's keychain. */
    readonly device: Device<NativeClient>;
    /** The authenticator of local clients and peers. */
    readonly authenticator: Authenticator;
    /** The events of this machine's own calls and telemetry. */
    readonly events: EventStore;
    /** The warm compilers compiling the builds. */
    readonly compilers: CompilerPool;
    /** The machine's local store of every build, which its builds compile into with their trees. */
    readonly packages: LocalBuildStore;
    /** The runtime of the spaces' instances. */
    readonly runtime: Runtime & { readonly close: () => Promise<void> };
    /** The open WebSockets of this machine's instances. */
    readonly hibernation: HibernationHost;
    /** The store of the device's partition: the computer's record and local objects. */
    readonly objects: ObjectStore;
    /** The device partition's workload, serving its services and its controllers. */
    readonly instance: WorkloadInstance;
    /** Find the machine's own service of a package in a partition, the device's by default. */
    readonly #services: (
        partition: string | undefined,
        packageId: string,
    ) => Pick<Server, "fetch"> | undefined;
    /** Close what the machine opened, last opened first. */
    readonly #stack: AsyncDisposableStack;
    /** Read the store of the machine's own partition while it serves. */
    readonly #partitionObjects: () => ObjectStore | undefined;

    /** Take a composed machine and what closes it. */
    private constructor(
        fields: Omit<
            Computer,
            "fetch" | "relay" | "machineObjects" | typeof Symbol.asyncDispose
        > & {
            readonly partitionObjects: () => ObjectStore | undefined;
            readonly services: (
                partition: string | undefined,
                packageId: string,
            ) => Pick<Server, "fetch"> | undefined;
        },
        stack: AsyncDisposableStack,
    ) {
        // set the composed services and what closes them
        this.machine = fields.machine;
        this.device = fields.device;
        this.authenticator = fields.authenticator;
        this.events = fields.events;
        this.compilers = fields.compilers;
        this.packages = fields.packages;
        this.runtime = fields.runtime;
        this.hibernation = fields.hibernation;
        this.objects = fields.objects;
        this.instance = fields.instance;
        this.#services = fields.services;
        this.#stack = stack;
        this.#partitionObjects = fields.partitionObjects;
    }

    /** Compose and start the machine over its state directory. */
    static async start(options: ComputerOptions): Promise<Computer> {
        // close what opened so far once a step fails, and let the tunnel's requests wait until started
        await using stack = new AsyncDisposableStack();
        const started = Promise.withResolvers<Computer>();

        // record the tunnel's statuses one after another, in the order the tunnel tells them
        let recorded = Promise.resolve();

        // refuse the held requests of a start that fails, which closes the tunnel
        started.promise.catch(() => undefined);
        stack.defer(() =>
            started.reject(
                MachineError.of("MACHINE_STOPPED", "the machine stopped while starting"),
            ),
        );
        const origin = `http://127.0.0.1:${options.port}`;
        const endpoint = options.endpoint ?? origin;

        // open the machine's database, the native device's objects, the sessions and the keys
        const state = await Computer.#openState(stack, options);
        const { machineId, database } = state;
        const keyring = await LocalKeyring.open(
            options.keychain(ROOT_KEYS),
            machineId,
            options.environment,
        );
        const identity = new MachineIdentity(
            machineId,
            options.keychain(MACHINE_KEYS),
            options.environment,
        );
        const events = await Computer.#openEvents(
            stack,
            options,
            database,
            "events",
            keyring,
            machineId,
        );

        // open the build cache over the shipped builds read in place, the compilers and the spaces' storage
        const builds = await Computer.#openCache(options, database);

        // read the builtins the distribution ships, which the machine links to the servers it loads
        const linked = DistributionManifest.builtins(options.shipped);

        // share the builtins with the servers the machine loads, from its compiled table or its files
        const program = await Program.current(
            async () =>
                new Map(
                    [...linked.values()].map((builtin) => [
                        builtin.package.name,
                        import.meta.dirname,
                    ]),
                ),
            options.environment.process.loader,
        );
        const compilers = await CompilerPool.open(builds.packages, options.toolchain);
        stack.defer(() => compilers[Symbol.asyncDispose]());
        const spaces = stack.use(
            await LocalSpaceStorage.open({
                directory: join(options.directory, "spaces"),
                endpoint: origin,
                keyring,
                // print the spaces' mail, as a computer sends none
                mail: mailConnector(
                    "print",
                    new PrintTransport((text) => options.environment.process.stdout.write(text)),
                ),
                environment: options.environment,
                report: (error) => options.report("spaces.failed", error),
            }),
        );

        // run the spaces' instances, their WebSockets open through the runtime, and load trusted servers through the Bun processes' builds
        const { runtime, processes } = Computer.#startRuntime(
            options,
            origin,
            { packages: builds.packages, program },
            () => machine,
        );
        stack.defer(() => runtime.close());
        const hibernation = new HibernationHost(
            (instanceId, path, request) =>
                runtime.deliver(
                    schema.identifier("instance").parse(instanceId),
                    path,
                    new Request(`http://instance${path}`, request),
                ),
            options.environment.telemetry,
        );

        // authenticate the owner, local clients and peers
        const authenticator = new Authenticator(
            machineId,
            state.device,
            {
                verify: (request, audience, space) => machine.verify(request, audience, space),
                requireStanding: (caller) => machine.requireStanding(caller),
                space: (id) => machine.space(id),
            },
            options.environment,
        );

        // serve the spaces while enrolled
        const machine: Machine = new Machine({
            id: machineId,
            environment: options.environment,
            identity,
            keyring,
            database,
            enrollment: (view) =>
                Computer.#enrollment(view ?? Snapshot.live(state.device.database), machineId),
            partition: () => routed.partition(),
            describe: async () =>
                Computer.#describe(
                    options,
                    state.device,
                    await Computer.#enrollment(Snapshot.live(state.device.database), machineId),
                ),
            spaces,
            cache: builds.cache,
            ...(options.releases === undefined ? {} : { releases: options.releases }),
            isBuilding: true,
            isEnforcingEntitlements: false,
            // serve accounts' hostnames on a cloud machine, which their DNS points at
            isServingHostnames: options.enrollment?.kind === "cloud",
            domain: LOOPBACK_DOMAIN,
            runtimes: [runtime, new ProcessRuntime()],
            hibernation,
            alarm: undefined,
            server: (manifest) => processes.server(manifest),
            linked,
            shipped: DistributionManifest.releases(options.shipped),
            skus: undefined,
            registry: options.registry,
            endpoint,
            version: MACHINE_PACKAGE.version,
            ...(options.relay === undefined
                ? {}
                : {
                      tunnel: {
                          ...(options.relay.kind === "url" ? { url: options.relay.url } : {}),
                          fetch: async (request: Request) => (await started.promise).relay(request),
                          report: (error: unknown) => options.report("tunnel.failed", error),
                          changed: (status: TunnelStatus) => {
                              recorded = recorded
                                  .then(async () => {
                                      const { objects } = await started.promise;
                                      await ComputerServer.link(objects, machineId, status);
                                  })
                                  .catch((error: unknown) =>
                                      options.report("computer.failed", error),
                                  );
                          },
                      },
                  }),
            authenticate: (request, audience, partition) =>
                authenticator.authenticate(request, audience, partition),
            authorizeSpace: (served) => authenticator.authorizeSpace(served),
            report: (error) => options.report("spaces.failed", error),
        });

        // serve this machine's objects and origins through the gateway
        const runner = new BuildRunner(
            machine,
            compilers,
            join(options.directory, "commits"),
            options.environment,
        );
        await options.environment.files.remove(join(options.directory, "commits"), {
            recursive: true,
            force: true,
        });
        const routed = await Computer.#startServers(stack, options, {
            ...state,
            machine,
            runner,
            compilers,
            authenticator,
            events,
            packages: builds.packages,
        });

        // announce what the machine runs again each time the device's distribution changes, until closed
        const announcing = new AbortController();
        const distributed = distribution.table[TABLE].sqlName;
        const { device } = state;
        const announced = device.database.log
            .waitUntil(
                async (committed) => {
                    if (committed.has(distributed) && (await machine.isEnrolled())) {
                        await machine.announce();
                    }

                    return false;
                },
                announcing.signal,
                device.database.state.commits.generation,
            )
            .catch((error: unknown) => options.report("machine.failed", error));
        stack.defer(async () => {
            announcing.abort();
            await announced;
        });

        // stop the spaces' instances first on close, while the spaces' gateways still send their calls
        stack.defer(() => runtime.close());

        // take the composed computer, and let the tunnel's held requests through
        const composed = new Computer(
            {
                ...state,
                ...routed,
                machine,
                authenticator,
                events,
                compilers,
                packages: builds.packages,
                runtime,
                hibernation,
            },
            stack.move(),
        );
        started.resolve(composed);

        return composed;
    }

    /** Erase a machine's state, refusing while it runs. */
    static async erase(
        directory: string,
        keychain: (service: string) => Keychain,
        environment: Environment<"files" | "server">,
    ): Promise<void> {
        // erase nothing before a first start
        const { files } = environment;
        if (!(await files.exists(directory))) {
            return;
        }

        // forget the logins' sessions and client keys and the machine's keys while the directory is locked
        {
            await using lock = await files.tryLock(join(directory, LOCK_FILE), "exclusive");
            if (lock === undefined) {
                throw MachineError.of(
                    "ALREADY_RUNNING",
                    "stop the Destack machine before erasing it",
                );
            }
            await Device.forget(directory, keychain, [MACHINE_KEYS, ROOT_KEYS], environment);
        }

        // remove the directory
        await files.remove(directory, { recursive: true, force: true });
    }

    /** Open the machine's database, its native device's database and the people's sessions. */
    static async #openState(stack: AsyncDisposableStack, options: ComputerOptions) {
        // open and migrate the machine's database
        const database: SqliteDatabase = await connectBunSqlite(
            join(options.directory, MACHINE_FILE),
            Computer.database,
            { environment: options.environment },
        );
        stack.defer(() => database.close());
        await database.migrate(Computer.database.tables);

        // open this computer's device, its local objects beside its logins, settings, visits and sign-ins
        const device = await Device.create(
            {
                home: options.directory,
                environment: options.environment,
                universe: options.universe,
            },
            { ...options.localObjects, login, deviceSetting, visit, signIn },
            options.keychain,
        );
        stack.use(device);
        const [existing] = await device.database
            .select({ id: computer.table.machineId })
            .from(computer.table);
        const machineId =
            options.machineId ?? existing?.id ?? Identifier.create("machine", options.environment);

        return { machineId, database, device };
    }

    /** Describe the computer as its machine reports it: system, start, update and host. */
    static async #describe(
        options: Pick<ComputerOptions, "process" | "environment">,
        device: Device,
        enrollment: MachineEnrollment | undefined,
    ): ReturnType<MachineOptions["describe"]> {
        // read the device's distribution, absent for a development build
        const [row] = await device.database.select().from(distribution.table);
        const update =
            row === undefined
                ? null
                : { installed: row.installed, staged: row.staged ?? null, status: row.status };

        // report the host's system, release and architecture, none for a cloud machine
        const { host } = options.environment;

        return {
            osVersion: host.version,
            startedAt: options.process.startedAt,
            update,
            host:
                enrollment?.kind === "cloud"
                    ? null
                    : {
                          platform: ComputerServer.platform(host.os),
                          architecture: host.architecture,
                      },
        };
    }

    /** Read the enrollment the computer's record states, declared or approved. */
    static async #enrollment(
        view: Pick<Snapshot, "read">,
        machineId: Identifier<"machine">,
    ): Promise<MachineEnrollment | undefined> {
        const row = await view.read(computer.table).findFirst({ where: { machineId } });

        return row?.enrollment ?? undefined;
    }

    /** Open the call events of a database, their segments in a directory of the machine's. */
    static async #openEvents(
        stack: AsyncDisposableStack,
        options: Pick<ComputerOptions, "directory" | "environment">,
        database: DatabaseConnection,
        directory: string,
        keyring: Keyring,
        machineId: Identifier<"machine">,
    ): Promise<EventStore> {
        const kinds: readonly EventKind[] = [call.eventKind()];
        const bucket = stack.use(
            await LocalBucket.open(
                join(options.directory, directory),
                machineId,
                options.environment,
            ),
        );

        return new EventStore({
            database,
            clock: options.environment.clock,
            kinds,
            files: () => bucket,
            personal: new DatabasePersonalKeyring(database, keyring, options.environment),
            settings: SettingValue.retention(database),
        });
    }

    /** Open the build cache: blobs below the machine's directory, shipped builds read in place. */
    static async #openCache(
        options: Pick<ComputerOptions, "directory" | "environment" | "layer" | "shipped">,
        database: SqliteDatabase,
    ): Promise<{ readonly cache: BuildCache; readonly packages: LocalBuildStore }> {
        // store the blobs in a directory, indexed in the machine's database, layered over the shipped builds
        const { environment } = options;
        const directory = join(options.directory, "builds");
        const storage = await LocalBlobStorage.open(join(directory, "files"), environment, {
            synchronous: "off",
        });
        const blobs = new BlobStore({ storage, database, environment });
        const packages = await LocalBuildStore.of({
            blobs,
            directory,
            environment,
            layer: options.layer,
        });

        // cache builds for the computer's runtimes, loading the shipped builtins in place of any other build of them
        const linked = DistributionManifest.builtins(options.shipped);

        return { cache: new BuildCache(packages, COMPUTER_RUNTIMES, linked), packages };
    }

    /** Start the instances' runtime, unless replaced, and the Bun processes loading builds. */
    static #startRuntime(
        options: ComputerOptions,
        origin: string,
        builds: { readonly packages: LocalBuildStore; readonly program: Program },
        machine: () => Machine,
    ): {
        readonly runtime: Runtime & { readonly close: () => Promise<void> };
        readonly processes: BunRuntime;
    } {
        // run each space's instances calling out through the egress of its space on this machine
        const identity = async (scope: Identifier<"space">) =>
            (await machine().space(scope)).host.identity();
        const egress = (scope: Identifier<"space">) =>
            `${SpaceMount.url(origin, scope)}${Egress.path}`;
        const processes = Computer.#startProcesses(
            options,
            origin,
            builds,
            egress,
            identity,
            machine,
        );

        return { runtime: options.runtime?.(egress, identity) ?? processes, processes };
    }

    /** Start the runtime running each instance as a Bun process of its own. */
    static #startProcesses(
        options: ComputerOptions,
        origin: string,
        { packages, program }: { readonly packages: LocalBuildStore; readonly program: Program },
        egress: (scope: Identifier<"space">) => string,
        identity: (scope: Identifier<"space">) => Promise<Deriver>,
        machine: () => Machine,
    ): BunRuntime {
        return new BunRuntime({
            program,
            environment: options.environment,
            report: (error) => options.report("runtime.failed", error),
            isolation: options.isolation,
            identity,
            directory: join(options.directory, "workers"),
            packages,
            egress,
            // serve an installation below this machine's origin, where its space runs
            url: async (scope, installation) => InstallationMount.url(origin, scope, installation),
            sampling: async (scope, installation) =>
                SettingValue.resolve(
                    Snapshot.live((await machine().space(scope)).database),
                    traceSampling,
                    { scope, installation },
                ),
            output: (running, line) => {
                // forward the line as a log record of the instance to its space's receivers
                const now = BigInt(options.environment.clock.now());
                const record = {
                    timeUnixNano: String(now * NANOSECONDS_PER_MILLISECOND),
                    eventName: OUTPUT_EVENT,
                    severityNumber: SeverityNumber.WARN,
                    body: { stringValue: line },
                    attributes: [{ key: "log.iostream", value: { stringValue: "stderr" } }],
                };
                const scope = { name: MACHINE_PACKAGE.name, version: MACHINE_PACKAGE.version };
                const exported = {
                    resourceLogs: [{ scopeLogs: [{ scope, logRecords: [record] }] }],
                };
                machine()
                    .space(running.scope)
                    .then((space) => space.telemetry(running, "logs", exported))
                    .catch((error: unknown) => options.report("runtime.output.failed", error));
            },
        });
    }

    /** Serve the device's partition always and the machine's while enrolled. */
    static async #startServers(
        stack: AsyncDisposableStack,
        options: ComputerOptions,
        opened: Pick<Computer, "machine" | "device" | "authenticator" | "events" | "compilers"> & {
            readonly runner: BuildRunner;
            readonly packages: LocalBuildStore;
        },
    ): Promise<
        Pick<Computer, "objects" | "instance"> & {
            readonly services: (
                partition: string | undefined,
                packageId: string,
            ) => Pick<Server, "fetch"> | undefined;
            readonly partition: () => Promise<AsyncDisposable>;
            readonly partitionObjects: () => ObjectStore | undefined;
        }
    > {
        // serve the device's calls beside its local objects
        const { machine, device } = opened;
        const deviceEvents = await Computer.#openEvents(
            stack,
            options,
            device.database,
            "device-events",
            machine.keyring,
            machine.id,
        );

        // create the machine package's server, handing each partition's store back as it starts
        const deviceStarted = Promise.withResolvers<ObjectStore>();
        let machineStarted: ObjectStore | undefined;
        const server = Computer.#server(options, opened, deviceEvents, {
            device: (store) => deviceStarted.resolve(store),
            machine: (store) => {
                machineStarted = store;
            },
        });

        // serve the device's partition from the start
        const instance = await Computer.#startWorkload(
            deviceWorkload,
            options,
            server,
            opened.authenticator,
        );
        const objects = await deviceStarted.promise;
        stack.defer(() => instance.close());

        // serve the machine's partition while it serves, its instance replaced at each start
        let partitioned: WorkloadInstance | undefined;
        const partition = async (): Promise<AsyncDisposable> => {
            const started = await Computer.#startWorkload(
                machineWorkload,
                options,
                server,
                opened.authenticator,
            );
            partitioned = started;

            return {
                [Symbol.asyncDispose]: async () => {
                    partitioned = undefined;
                    machineStarted = undefined;
                    await started.close();
                },
            };
        };

        // route each partition's requests to its store
        const services = Computer.#servicesOf((partitionId) => {
            // route the device's partition, the default, to its store
            if (partitionId === undefined || partitionId === device.id) {
                return instance;
            }
            // route the machine's partition to its store once enrolled
            else if (partitionId === machine.id && partitioned !== undefined) {
                return partitioned;
            }
            // refuse the machine's partition before it enrolls
            else if (partitionId === machine.id) {
                throw MachineError.unenrolled("storing checkouts and previews");
            }

            return undefined;
        });

        return { objects, instance, services, partition, partitionObjects: () => machineStarted };
    }

    /** Create the machine package's server over this machine's state, handing each store back. */
    static #server(
        options: ComputerOptions,
        opened: Pick<Computer, "machine" | "device" | "events" | "compilers"> & {
            readonly runner: BuildRunner;
        },
        deviceEvents: EventStore,
        receive: ComputerServer["receive"],
    ): ComputerServer {
        const { machine, device } = opened;

        return new ComputerServer({
            device,
            callKey: () => Computer.#callKey(machine),
            session: (loginId) => device.session(loginId),
            machine,
            runner: opened.runner,
            shutdown: () => options.shutdown(),
            build: {
                compilers: opened.compilers,
                host: options.environment,
                connect: (handle, loginId, spaceId) =>
                    Computer.#connect(handle, options, opened, loginId, spaceId),
            },
            events: opened.events,
            deviceEvents,
            localObjects: [...Object.values(options.localObjects), visit, signIn],
            process: options.process,
            platform: options.environment.host,
            enrollment: options.enrollment,
            distribution: options.distribution,
            receive,
        });
    }

    /** Derive the key this machine fingerprints sensitive request inputs under. */
    static #callKey(machine: Machine): Promise<CryptoKey> {
        return CallKey.of(machine.keyring, machine.environment.crypto);
    }

    /** Start one of the computer's partitions with its controllers. */
    static #startWorkload(
        workload: typeof deviceWorkload,
        options: ComputerOptions,
        server: ComputerServer,
        authenticator: Authenticator,
    ): Promise<WorkloadInstance> {
        const { machine, device } = server;

        return WorkloadInstance.start(workload, {
            environment: options.environment,
            resources: new ResourceContext().bind(computerServer, server),
            callKey: () => Computer.#callKey(machine),
            report: (error) => options.report("machine.failed", error),
            serverOptions: (handle) => ({
                environment: options.environment,
                scope: workload === deviceWorkload ? device.id : machine.id,
                drainTimeout: DRAIN_TIMEOUT_MILLISECONDS,
                authenticate: (request) =>
                    workload === deviceWorkload
                        ? authenticator.authenticateDevice(request, handle.id)
                        : authenticator.authenticate(request, handle.id, machine.id),
                authorizeMachine: (hosted) => authenticator.authorizeMachine(hosted),
            }),
        });
    }

    /** Connect to a package serving a space as a person signed in on this computer. */
    static async #connect<Handle extends PackageHandle>(
        handle: Handle,
        options: Pick<ComputerOptions, "environment">,
        opened: Pick<Computer, "machine" | "device">,
        loginId: Identifier<"login">,
        spaceId: Identifier<"space">,
    ): Promise<ClientOf<Handle>> {
        // call the package at the machine serving the space as the person
        const signedIn = await opened.device.signedIn(loginId);
        const located = await opened.machine.locate(spaceId);
        if (located.placement.machine !== opened.machine.id) {
            return signedIn.person.package(handle, located.url, spaceId);
        }

        // call the package mounted here as the person's own client
        const server = opened.machine.package(handle);
        if (server === undefined) {
            throw MachineError.unenrolled("serving spaces");
        }

        return createClient<Handle["router"]>(
            handle,
            {
                url: "http://127.0.0.1/",
                fetch: async (request: Request) => {
                    // prove the person's client afresh on each call
                    const headers = new Headers(request.headers);
                    headers.set("authorization", `Bearer ${await signedIn.prove()}`);

                    return server.fetch(new Request(request, { headers }));
                },
            },
            options.environment,
        );
    }

    /** Find the services of the partition a request addresses, each partition's in its store. */
    static #servicesOf(
        instanceOf: (partition: string | undefined) => WorkloadInstance | undefined,
    ): (partition: string | undefined, packageId: string) => Pick<Server, "fetch"> | undefined {
        return (partition, packageId) => {
            const instance = instanceOf(partition);
            const handle = instance?.packages.find((each) => each.id === packageId);

            return instance === undefined || handle === undefined
                ? undefined
                : Computer.#serving(instance, handle);
        };
    }

    /** Read the store of the machine's own partition, refusing before the machine serves it. */
    machineObjects(): ObjectStore {
        const store = this.#partitionObjects();
        if (store === undefined) {
            throw MachineError.unenrolled("serving the machine's partition");
        }

        return store;
    }

    /** Read the machine's configuration from its release channel and environment. */
    static configure(host: Environment<"process" | "host">): Pick<
        ComputerOptions,
        "directory" | "universe" | "port" | "relay"
    > & {
        /** The directory of the distribution the computer ships: its manifest and its layer. */
        readonly shipped: string;
    } {
        // take the universe the environment names, else the stage's
        const environment = host.process.variables;
        const home = StageHome.current(host);
        const { universe } = home;
        if (!URL.canParse(universe)) {
            throw MachineError.of(
                "CONFIGURATION_INVALID",
                `DESTACK_UNIVERSE is not a URL: ${universe}`,
            );
        }

        // require a nonzero port
        const configured = environment["DESTACK_PORT"];
        const port = configured === undefined ? MACHINE_PORT : Number(configured);
        if (!/^\d+$/u.test(configured ?? String(MACHINE_PORT)) || port < 1 || port > 65535) {
            throw MachineError.of(
                "CONFIGURATION_INVALID",
                `DESTACK_PORT is not a port: ${configured}`,
            );
        }

        // open a tunnel to the relay at a URL, else to the account residency's in production
        const url = environment["DESTACK_RELAY"];
        if (url !== undefined && !URL.canParse(url)) {
            throw MachineError.of("CONFIGURATION_INVALID", `DESTACK_RELAY is not a URL: ${url}`);
        }
        const relay: ComputerOptions["relay"] =
            url !== undefined
                ? { kind: "url", url }
                : home.stage.name === "production"
                  ? { kind: "residency" }
                  : undefined;

        // ship the distribution the environment names, else the one in a release's resources
        const { executable, runtime } = host.process;
        const bundled = runtime.isStandalone
            ? join(Distribution.resources(executable), DISTRIBUTION_DIRECTORY)
            : undefined;
        const shipped = environment["DESTACK_DISTRIBUTION"] ?? bundled;
        if (shipped === undefined) {
            throw MachineError.of(
                "CONFIGURATION_INVALID",
                "DESTACK_DISTRIBUTION is not set outside a release",
            );
        }

        return {
            directory: home.directory,
            universe,
            port,
            ...(relay === undefined ? {} : { relay }),
            shipped,
        };
    }

    /** Choose where the machine stores its keys: the keychain, else files on a headless Linux. */
    static keychainStore(
        directory: string,
        environment: Pick<ComputerOptions["environment"], "host" | "process">,
    ): KeychainStore {
        // store the keys in files on a Linux computer without a session bus, such as a server
        const isHeadless =
            environment.host.os === "linux" &&
            environment.process.variables["DBUS_SESSION_BUS_ADDRESS"] === undefined;

        return isHeadless
            ? { kind: "file", directory: join(directory, KEYCHAIN_DIRECTORY) }
            : { kind: "system" };
    }

    /** Serve this computer until shutdown, with its device's local objects. */
    static async run(
        bundle: Pick<ComputerOptions, "localObjects" | "registry">,
        environment: ComputerOptions["environment"],
        configuration = Computer.configure(environment),
    ): Promise<void> {
        // take the directory's database exclusively
        const { port } = configuration;
        const { files } = environment;
        const machineProcess = new MachineProcess(configuration.directory, environment);
        await machineProcess.createDirectory();
        const directory = await files.realPath(configuration.directory);
        await using cleanup = new AsyncDisposableStack();
        const lock = await files.tryLock(join(directory, LOCK_FILE), "exclusive");
        if (!lock) {
            throw MachineError.of("ALREADY_RUNNING", "the Destack machine is already running");
        }
        cleanup.defer(() => lock.close());

        // choose the keys' store, and report it in the machine's log
        const store = Computer.keychainStore(directory, environment);
        environment.process.stderr.writeSync(`keychain: ${store.kind}\n`);

        // record the process's telemetry from its start: in its own log, and in its owner's home once enrolled
        let running: Computer | undefined;
        const telemetry = await Computer.#startTelemetry(environment, () => running?.machine);
        const instruments = TelemetryScope.of(MACHINE_PACKAGE, telemetry);
        const recorded = { ...environment, telemetry };

        // compose the machine with the system's keychains, this channel's distribution and its toolchain
        const stopping = new AbortController();
        const home = StageHome.current(environment);
        const installed = home.distribution(environment);
        const toolchain = environment.process.runtime.isStandalone
            ? () => installed.distribution.toolchain()
            : async () => Toolchain.workspace();
        const started = await Computer.start({
            isolation: "sandbox",
            directory,
            environment: recorded,
            universe: configuration.universe,
            port,
            ...(configuration.relay === undefined ? {} : { relay: configuration.relay }),
            ...bundle,
            shipped: await BunDistribution.read(configuration.shipped, files),
            // ship no releases: the cache pulls each from its forge on first use
            layer: BuildLayer.empty(),
            keychain: (service) => new BunKeychain(service, files, store),
            process: {
                version: MACHINE_PACKAGE.version,
                pid: environment.process.id,
                startedAt: environment.clock.now(),
            },
            ...(home.stage.name === "local" ? {} : { distribution: installed }),
            toolchain,
            report: (event, error) =>
                instruments.captureException(error, { isEscaped: true, tags: { event } }),
            shutdown: () => stopping.abort(),
        });
        running = started;
        cleanup.use(started);
        cleanup.defer(() => stopping.abort());

        // flush the telemetry before the machine closes, its home's export going through the machine
        cleanup.defer(() => telemetry.shutdown());

        // note the machine's start and stop in its telemetry
        const note = (event: string, attributes: Readonly<Record<string, number>> = {}) =>
            instruments.log.info(event, attributes);

        // publish the endpoint once the listener opens
        await serveProcess({
            endpoints: [
                {
                    hostname: "127.0.0.1",
                    port,
                    fetch: (request) => started.fetch(request),
                    hibernation: started.hibernation,
                },
            ],
            environment,
            signal: stopping.signal,
            shutdown: () => stopping.abort(),
            close: async () => {
                // note the stop while the machine's telemetry still runs, then close everything
                note("machine.stopped");
                await cleanup.disposeAsync();
            },
            ready: async ([address]) => {
                // require the listener's address
                if (address === undefined) {
                    throw MachineError.of(
                        "LISTENER_MISSING",
                        "the machine does not listen on any address",
                    );
                }

                // publish it to local clients
                const listening = Number(address.port);
                await machineProcess.publish(cleanup, {
                    port: listening,
                    token: started.authenticator.token,
                    machine: started.machine.id,
                    device: started.device.id,
                });
                note("machine.started", {
                    "process.pid": environment.process.id,
                    "server.port": listening,
                });
            },
        });
    }

    /** Serve one of a workload instance's packages through the gateway. */
    static #serving(instance: WorkloadInstance, handle: PackageHandle): Pick<Server, "fetch"> {
        return { fetch: (request, caller) => instance.fetch(handle, request, caller) };
    }

    /** Route a request on this machine's listener. */
    async fetch(request: Request): Promise<Response> {
        // find the machine's own service a loopback request asks for
        const url = new URL(request.url);
        const isLoopback = url.hostname === "127.0.0.1";
        const isMounted = isLoopback && PackageMount.parse(url.pathname) !== undefined;
        const isOwn = isMounted && !(await this.machine.isSpaceRequest(request));
        const own = isOwn
            ? await this.#own(request).then(
                  (found) => ({ found }),
                  (error: unknown) => ({ error }),
              )
            : undefined;

        // refuse a service of a partition this machine does not serve yet
        if (own !== undefined && "error" in own) {
            return refusal(own.error, this.machine.environment.telemetry);
        }
        // serve one of the machine's own services
        else if (own?.found !== undefined) {
            return own.found.server.fetch(own.found.request);
        }
        // serve a name at any host but loopback, the endpoint and its origins
        else if (
            !HostAddress.isLoopbackName(url.hostname) &&
            url.hostname !== new URL(this.machine.endpoint).hostname
        ) {
            return this.machine.serveName(request);
        }

        // hand anything else to the spaces: their origins, services, workloads' calls and buckets
        return this.machine.serve(request);
    }

    /** Route a request the relay forwards through this machine's tunnel. */
    async relay(request: Request): Promise<Response> {
        // serve a name the relay routes here, else the machine's peers
        const url = new URL(request.url);
        if (MachineAddress.parse(url.hostname, DOMAINS.machine) === undefined) {
            return this.machine.serveName(request);
        }

        // find the machine partition's own service at its name
        const own = await this.#own(request).then(
            (found) => ({ found }),
            (error: unknown) => ({ error }),
        );
        if ("error" in own) {
            return refusal(own.error, this.machine.environment.telemetry);
        }
        // serve the machine's own service
        else if (own.found?.isMachine === true) {
            return own.found.server.fetch(own.found.request);
        }
        // refuse another partition's service at the machine's name
        else if (own.found !== undefined) {
            return new Response(null, { status: 404 });
        }

        return this.machine.serve(request);
    }

    /** Find the machine's own service below whose mount a request falls, in its partition. */
    async #own(request: Request): Promise<
        | {
              readonly server: Pick<Server, "fetch">;
              readonly request: Request;
              readonly isMachine: boolean;
          }
        | undefined
    > {
        // find the mount and the partition below it, leaving the request whole for a service it misses
        const mounted = PackageMount.parse(new URL(request.url).pathname);
        if (mounted === undefined) {
            return undefined;
        }
        const partition = await PartitionRequest.of(request, mounted.path);
        const server = this.#services(partition, mounted.packageId);
        if (server === undefined) {
            return undefined;
        }

        // take the request below the mount for the service
        const routed = present(PackageMount.of(request), "the request below its mount");
        const isMachine = MACHINE_IDENTIFIER.safeParse(partition).success;

        return { server, request: routed.request, isMachine };
    }

    /** Start a machine process's telemetry: its own log, and its owner's home once enrolled. */
    static async #startTelemetry(
        environment: Environment<"process">,
        machine: () => Machine | undefined,
    ): Promise<Telemetry> {
        // post each export to the telemetry of the owner's home where its machine answers, nothing before it enrolls
        const target = {
            endpoint: `${OWN_TELEMETRY_ORIGIN}${OTLP_ORIGIN_PATH}`,
            fetch: async (request: Request) => {
                // locate the owner's home and send as this machine bound to it, below the path
                const own = present(machine(), "the enrolled machine");
                const home = await own.home();
                const { url } = await own.directory.locate(home, own.clock.now());
                const { pathname } = new URL(request.url);
                const send = own.fetch(stackPackage.id, { partition: home });

                return send(new Request(`${SpaceMount.url(url, home)}${pathname}`, request));
            },
        };
        const exporter = new OtlpExporter(
            async (signal, body) => {
                const own = machine();
                if (own !== undefined && (await own.isEnrolled())) {
                    await OtlpExporter.post(target, signal, body);
                }
            },
            // print a failure of the telemetry with its causes
            (error) => environment.process.stderr.writeSync(`telemetry: ${inspect(error)}\n`),
        );

        // write every record to the machine's own log too, beside its owner's home
        const options = exporter.options(MACHINE_PACKAGE);
        const local = new LineExporter((line) => environment.process.stderr.writeSync(line));

        return startTelemetry(
            {
                ...options,
                signals: (meters) => {
                    const signals = options.signals(meters);
                    const processors = [...(signals.logs?.processors ?? []), local.processor];

                    return { ...signals, logs: { ...signals.logs, processors } };
                },
            },
            environment,
        );
    }

    /** Stop serving and close what the machine opened, last opened first. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#stack.disposeAsync();
    }
}
