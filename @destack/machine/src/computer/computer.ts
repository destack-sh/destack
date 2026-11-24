import type { TunnelStatus } from "@destack/tunnel";
import { BunDistribution, Program } from "@destack/package/bun";
import {
    PackageMount,
    refusal,
    tablesOf,
    call,
    Egress,
    InstallationMount,
    type Package,
    SpaceMount,
    CallKey,
    type ObjectType,
} from "@destack/object";
import objectPackage from "@destack/object/package";
import { checkout } from "./checkout/index.ts";
import { defineDatabase, TABLE } from "@destack/db";
import { BUILD_STORE_TABLES } from "@destack/build/store";
import { join } from "node:path";
import { Distribution, DISTRIBUTION_DIRECTORY } from "@destack/update";
import { Toolchain } from "@destack/check/toolchain";
import { distribution } from "@destack/update/object";
import { MachineKey } from "@destack/account/client";
import { login, type NativeClient } from "@destack/account/native";
import type { Server, ObjectStore } from "@destack/object/server";
import { LocalBlobStorage, LocalBucket } from "@destack/bucket/bun";
import { CompilerPool, LocalBuildStore } from "@destack/build";
import { BuildLayer } from "@destack/package/manifest";
import { BlobStore, type DatabaseConnection, type Snapshot } from "@destack/db/database";
import { connectBunSqlite } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import type { EventKind } from "@destack/event";
import { DatabasePersonalKeyring, EventStore } from "@destack/event/store";
import { type Deriver, type Keychain, type Keyring, LocalKeyring } from "@destack/identity";
import { BunKeychain, type KeychainStore } from "@destack/identity/bun";
import { mailConnector } from "@destack/mail/transport";
import { PrintTransport } from "@destack/mail/print";
import { traceSampling } from "@destack/stack/setting";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, present, schema } from "@destack/schema";
import {
    DISTRIBUTION_LAYER,
    DistributionManifest,
    DistributionPlatform,
    OriginPath,
    type PackageRelease,
} from "@destack/package";
import { serveProcess } from "@destack/object/bun";
import { type Hibernation, HibernationHost, WorkloadInstance } from "@destack/object/workload";
import { deviceSetting, SettingValue } from "@destack/setting/object";
import { type BunIsolation, BunRuntime } from "../bun/index.ts";
import { ProcessRuntime } from "../runtime/process.ts";
import type { Runtime } from "../runtime/runtime.ts";
import { SeverityNumber, TelemetryScope } from "@destack/telemetry";
import { startTelemetry } from "@destack/telemetry/bun";
import { OtlpExporter } from "@destack/telemetry/otlp";
import stackPackage from "@destack/stack/package";
import type { DistributionHost } from "@destack/update/server";
import {
    DRAIN_TIMEOUT_MILLISECONDS,
    Machine,
    type MachineOptions,
    OLDEST_DESTACK,
} from "../machine/index.ts";
import { Authenticator } from "./device/authentication/index.ts";
import { Device } from "./device/device.ts";
import { MachineError } from "../error/index.ts";
import {
    InstallationOrigin,
    LOOPBACK_DOMAIN,
    S3Origin,
    MachineAddress,
} from "@destack/account/address";
import { ComputerServer } from "./server/index.ts";
import { LocalSpaceStorage } from "./storage.ts";
import { computer, type MachineEnrollment, signIn, visit } from "./device/object/index.ts";
import { MachineProcess, LOCK_FILE, UniverseHome } from "./process/index.ts";
import { Builder } from "./build.ts";
import { Edit } from "./edit.ts";
import { Runner } from "./runner.ts";
import { EDIT_PUSH } from "../installation/edit.ts";
import { BUILD_COMPILE } from "../installation/compile.ts";
import { computerServer, machineWorkload, deviceWorkload } from "./server/workload.ts";
import { Environment, HostAddress } from "@destack/environment";
import { LineExporter } from "@destack/environment/line";
import { DEPLOYING_WAIT_MILLISECONDS, PartitionRequest, Space } from "../space/index.ts";

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
    /** Run the machine's instances, calling out through an egress and reading its builds. */
    readonly runtime?: (
        egress: (scope: Identifier<"space">) => string,
        identity: (scope: Identifier<"space">) => Promise<Deriver>,
        packages: LocalBuildStore,
    ) => Runtime & { readonly close: () => Promise<void> };
    /** The installed distribution the machine stages and activates. */
    readonly distribution?: DistributionHost;
    /** Read the toolchain the compilers run, once they first need it. */
    readonly toolchain: () => Promise<Toolchain>;
    /** Stop the machine at a client's request. */
    readonly shutdown: () => void;
}

/** The Destack process on a computer: its machine, native device, view origins and gateway. */
export class Computer implements AsyncDisposable {
    /** The seconds a request may idle at a machine: its longest hold plus one. */
    static readonly idleTimeout = DEPLOYING_WAIT_MILLISECONDS / 1000 + 1;

    /** The database of the machine's own partition on a computer. */
    static readonly database = defineDatabase({
        name: "machine",
        ...tablesOf({
            packages: [objectPackage, { objects: { checkout }, tables: [] }],
            follows: [],
            // index the machine's build store beside its objects
            tables: BUILD_STORE_TABLES,
        }),
        // replicate the universe's directory and the steps placed on this machine
        replicated: [
            ...Space.functions.flatMap(({ workflow }) =>
                workflow.placed.objects.map((object) => object.table),
            ),
            ...Machine.replicated,
        ],
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
    static start(options: ComputerOptions): Promise<Computer> {
        // boot the computer in its span, from its state to serving
        return TelemetryScope.of(MACHINE_PACKAGE, options.environment.telemetry).span(
            "destack.computer.start",
            {},
            async () => {
                // close what opened so far once a step fails, and let the tunnel's requests wait until started
                await using stack = new AsyncDisposableStack();
                const started = Promise.withResolvers<Computer>();
                const partitions = new Map<string, ComputerPartition>();

                // record the tunnel's statuses one after another, in the order the tunnel tells them
                let recorded = Promise.resolve();

                // refuse the held requests of a start that fails, which closes the tunnel
                started.promise.catch(() => undefined);
                stack.defer(() =>
                    started.reject(
                        MachineError.of(
                            "MACHINE_UNAVAILABLE",
                            "the machine is unavailable: it stopped while starting",
                        ),
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
                const identity = new MachineKey(
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

                // read the releases the distribution ships on the host, its builtins linked to the servers the machine loads
                const platform = DistributionPlatform.of(options.environment.host);
                const shipped = DistributionManifest.releases(options.shipped, platform);
                const linked = DistributionManifest.builtins(options.shipped, platform);

                // open the machine's builds over the shipped builds read in place, the compilers and the spaces' storage
                const builds = stack.use(await Computer.#openBuilds(options, database));

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
                const compilers = await CompilerPool.open(builds, options.toolchain);
                stack.defer(() => compilers[Symbol.asyncDispose]());
                const spaces = stack.use(
                    await LocalSpaceStorage.open({
                        directory: join(options.directory, "spaces"),
                        endpoint: origin,
                        keyring,
                        // print the spaces' mail, as a computer sends none
                        mail: mailConnector(
                            "print",
                            new PrintTransport((text) =>
                                options.environment.process.stdout.write(text),
                            ),
                        ),
                        environment: options.environment,
                    }),
                );

                // run the spaces' instances, their WebSockets open through the runtime, and load trusted servers through the Bun processes' builds
                const { runtime, processes } = Computer.#startRuntime(
                    options,
                    origin,
                    {
                        packages: builds,
                        program,
                        builtins: [...linked.values()],
                    },
                    () => machine,
                );
                stack.defer(() => runtime.close());
                // authenticate the owner, local clients and peers
                const authenticator = new Authenticator(
                    machineId,
                    state.device,
                    {
                        verify: (request, audience, space) =>
                            machine.verify(request, audience, space),
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
                        Computer.#enrollment(view ?? state.device.database.snapshot(), machineId),
                    partition: () => routed.partition(),
                    describe: async () =>
                        Computer.#describe(
                            options,
                            state.device,
                            await Computer.#enrollment(state.device.database.snapshot(), machineId),
                        ),
                    volumes: spaces,
                    builds,
                    targets: COMPUTER_RUNTIMES,
                    isRunner: true,
                    isEnforcingEntitlements: false,
                    // serve accounts' hostnames on a cloud machine, which their DNS points at
                    isServingHostnames: options.enrollment?.kind === "cloud",
                    domain: LOOPBACK_DOMAIN,
                    runtimes: [runtime, new ProcessRuntime()],
                    instances: async (instanceId, path, request) => {
                        // hand a partition's socket event to its servers, and an installation's to its runtime
                        const partition = partitions.get(instanceId);
                        const delivered = new Request(`http://instance${path}`, request);
                        if (partition === undefined) {
                            return runtime.deliver(
                                schema.identifier("instance").parse(instanceId),
                                path,
                                delivered,
                            );
                        }
                        await partition.hibernation.receive(delivered, partition.instance.sockets);

                        return new Response(null, { status: 204 });
                    },
                    alarm: undefined,
                    server: (image) => processes.server(image),
                    linked,
                    shipped,
                    endpoint,
                    destack: { oldest: OLDEST_DESTACK, newest: MACHINE_PACKAGE.version },
                    ...(options.relay === undefined
                        ? {}
                        : {
                              tunnel: {
                                  ...(options.relay.kind === "url"
                                      ? { url: options.relay.url }
                                      : {}),
                                  fetch: async (request: Request) =>
                                      (await started.promise).relay(request),
                                  report: (error: unknown) =>
                                      Computer.capture(
                                          options.environment.telemetry,
                                          "tunnel.failed",
                                          error,
                                      ),
                                  changed: (status: TunnelStatus) => {
                                      recorded = recorded
                                          .then(async () => {
                                              const { objects } = await started.promise;
                                              await ComputerServer.link(objects, machineId, status);
                                          })
                                          .catch((error: unknown) =>
                                              Computer.capture(
                                                  options.environment.telemetry,
                                                  "computer.failed",
                                                  error,
                                              ),
                                          );
                                  },
                              },
                          }),
                    authenticate: (request, audience, partition) =>
                        authenticator.authenticate(request, audience, partition),
                    authorizeSpace: (served) => authenticator.authorizeSpace(served),
                });

                // build on this machine
                const builder = new Builder(
                    machine,
                    compilers,
                    join(options.directory, "commits"),
                    options.environment,
                );

                // run the steps placed on this machine by their functions
                const runner = new Runner(
                    machine,
                    {
                        [EDIT_PUSH.name]: {
                            workflow: EDIT_PUSH.workflow,
                            run: (input, context) =>
                                new Edit(EDIT_PUSH.workflow.input.parse(input), context).push(),
                        },
                        [BUILD_COMPILE.name]: {
                            workflow: BUILD_COMPILE.workflow,
                            run: (input, context) =>
                                builder.build(
                                    BUILD_COMPILE.workflow.input.parse(input).build,
                                    context,
                                ),
                        },
                    },
                    { builder, environment: options.environment },
                    join(options.directory, "runs"),
                );

                // clear the working directories an earlier process left
                for (const directory of ["commits", "runs"]) {
                    await options.environment.files.remove(join(options.directory, directory), {
                        recursive: true,
                        force: true,
                    });
                }

                // serve this machine's objects and origins through the gateway
                const routed = await Computer.#startServers(stack, options, partitions, {
                    ...state,
                    machine,
                    runner,
                    compilers,
                    authenticator,
                    events,
                    packages: builds,
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
                    .catch((error: unknown) =>
                        Computer.capture(options.environment.telemetry, "machine.failed", error),
                    );
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
                        packages: builds,
                        runtime,
                        hibernation: present(machine.hibernation, "the machine's sockets"),
                    },
                    stack.move(),
                );
                started.resolve(composed);

                return composed;
            },
        );
    }

    /** Erase a machine's state, refusing while it runs. */
    static erase(
        directory: string,
        keychain: (service: string) => Keychain,
        environment: Environment<"files" | "server">,
    ): Promise<void> {
        // erase the computer's state in its span
        return TelemetryScope.of(MACHINE_PACKAGE, environment.telemetry).span(
            "destack.computer.erase",
            {},
            async () => {
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
                            "MACHINE_BLOCKED",
                            "cannot erase the Destack machine: it is running, so stop it first",
                        );
                    }
                    await Device.forget(
                        directory,
                        keychain,
                        [MACHINE_KEYS, ROOT_KEYS],
                        environment,
                    );
                }

                // remove the directory
                await files.remove(directory, { recursive: true, force: true });
            },
        );
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
            environment: options.environment,
            kinds,
            files: () => bucket,
            personal: new DatabasePersonalKeyring(database, keyring, options.environment),
            settings: SettingValue.retention(database),
        });
    }

    /** Open the machine's builds: blobs below its directory, shipped builds read in place. */
    static async #openBuilds(
        options: Pick<ComputerOptions, "directory" | "environment" | "layer">,
        database: SqliteDatabase,
    ): Promise<LocalBuildStore> {
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

        return packages;
    }

    /** Start the instances' runtime, unless replaced, and the Bun processes loading builds. */
    static #startRuntime(
        options: ComputerOptions,
        origin: string,
        builds: {
            readonly packages: LocalBuildStore;
            readonly program: Program;
            readonly builtins: readonly PackageRelease[];
        },
        machine: () => Machine,
    ): {
        readonly runtime: Runtime & { readonly close: () => Promise<void> };
        readonly processes: BunRuntime;
    } {
        // run each space's instances calling out through the egress of its space on this machine
        const identity = async (scope: Identifier<"space">) =>
            (await machine().space(scope)).identity();
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

        // run them in the given runtime, else in the Bun processes
        const runtime = options.runtime?.(egress, identity, builds.packages) ?? processes;

        return { runtime, processes };
    }

    /** Start the runtime running each instance as a Bun process of its own. */
    static #startProcesses(
        options: ComputerOptions,
        origin: string,
        {
            packages,
            program,
            builtins,
        }: {
            readonly packages: LocalBuildStore;
            readonly program: Program;
            readonly builtins: readonly PackageRelease[];
        },
        egress: (scope: Identifier<"space">) => string,
        identity: (scope: Identifier<"space">) => Promise<Deriver>,
        machine: () => Machine,
    ): BunRuntime {
        return new BunRuntime({
            program,
            environment: options.environment,
            isolation: options.isolation,
            identity,
            directory: join(options.directory, "workers"),
            packages,
            builtins,
            egress,
            // serve an installation below this machine's origin, where its space runs
            url: async (scope, installation) => InstallationMount.url(origin, scope, installation),
            // sample each installation as its settings set, batching as the machine's own variables set
            workloadTelemetry: async (scope, installation) => ({
                sampling: await SettingValue.resolve(
                    (await machine().space(scope)).database.snapshot(),
                    traceSampling,
                    { scope, installation },
                ),
                delay: OtlpExporter.delay(options.environment.process.variables),
            }),
            output: (running, line) => {
                // write the line to the machine's own log, as it writes its own records
                options.environment.process.stderr.writeSync(`${line}\n`);

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
                    .catch((error: unknown) =>
                        Computer.capture(
                            options.environment.telemetry,
                            "runtime.output.failed",
                            error,
                        ),
                    );
            },
        });
    }

    /** Serve the device's partition always and the machine's while enrolled. */
    static async #startServers(
        stack: AsyncDisposableStack,
        options: ComputerOptions,
        partitions: Map<string, ComputerPartition>,
        opened: Pick<Computer, "machine" | "device" | "authenticator" | "events" | "compilers"> & {
            readonly runner: Runner;
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
        const devicePartition = await Computer.#startWorkload(
            deviceWorkload,
            options,
            server,
            opened.authenticator,
        );
        const { instance } = devicePartition;
        partitions.set(device.id, devicePartition);
        const objects = await deviceStarted.promise;
        stack.defer(async () => {
            // release its sockets at the machine's host, their clients resuming, then close it
            partitions.delete(device.id);
            machine.hibernation?.release(device.id);
            await instance.close();
        });

        // serve the machine's partition while it serves, its instance replaced at each start
        let partitioned: WorkloadInstance | undefined;
        const partition = async (): Promise<AsyncDisposable> => {
            // start it, routing its requests and socket events to it
            const started = await Computer.#startWorkload(
                machineWorkload,
                options,
                server,
                opened.authenticator,
            );
            partitioned = started.instance;
            partitions.set(machine.id, started);

            return {
                [Symbol.asyncDispose]: async () => {
                    // stop routing to it, release its sockets at the machine's host, then close it
                    partitioned = undefined;
                    machineStarted = undefined;
                    partitions.delete(machine.id);
                    machine.hibernation?.release(machine.id);
                    await started.instance.close();
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
                throw MachineError.unenrolled("storing checkouts");
            }

            return undefined;
        });

        return { objects, instance, services, partition, partitionObjects: () => machineStarted };
    }

    /** Create the machine package's server over this machine's state, handing each store back. */
    static #server(
        options: ComputerOptions,
        opened: Pick<Computer, "machine" | "device" | "events" | "compilers"> & {
            readonly runner: Runner;
        },
        deviceEvents: EventStore,
        receive: ComputerServer["receive"],
    ): ComputerServer {
        const { machine, device } = opened;

        return new ComputerServer({
            device,
            callKey: () => Computer.#callKey(machine),
            launcher: (loginId) => device.launcher(loginId, (request) => machine.send(request)),
            machine,
            runner: opened.runner,
            shutdown: () => options.shutdown(),
            build: {
                compilers: opened.compilers,
                host: options.environment,
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

    /** Start one of the computer's partitions, its sockets kept at the machine's host. */
    static async #startWorkload(
        workload: typeof deviceWorkload,
        options: ComputerOptions,
        server: ComputerServer,
        authenticator: Authenticator,
    ): Promise<ComputerPartition> {
        // keep the partition's sockets at the machine's host under the partition, sending through it in this process
        const { machine, device } = server;
        const id = workload === deviceWorkload ? device.id : machine.id;
        const hibernation = present(machine.hibernation, "the machine's sockets").instance(id);

        // start it
        const instance = await WorkloadInstance.start(workload, {
            environment: options.environment,
            hibernation,
            resources: new ResourceContext().bind(computerServer, server),
            callKey: () => Computer.#callKey(machine),
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

        return { instance, hibernation };
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

    /** Open the layer a distribution declares, else an empty one. */
    static async #layer(
        directory: string,
        shipped: DistributionManifest,
        environment: Environment<"files">,
    ): Promise<BuildLayer> {
        return shipped.isLayered
            ? await BuildLayer.open(join(directory, DISTRIBUTION_LAYER), environment)
            : BuildLayer.empty();
    }

    /** Read the machine's configuration from its release channel and environment. */
    static configure(host: Environment<"process" | "host">): Pick<
        ComputerOptions,
        "directory" | "universe" | "port" | "relay"
    > & {
        /** The directory of the distribution the computer ships: its manifest and its layer. */
        readonly shipped: string;
    } {
        // take the universe the environment names, else the universe's
        const environment = host.process.variables;
        const home = UniverseHome.current(host);
        const { universe } = home;
        if (!URL.canParse(universe)) {
            throw MachineError.of(
                "CONFIGURATION_BROKEN",
                `DESTACK_UNIVERSE_ORIGIN is not a URL: '${universe}'`,
            );
        }

        // require a nonzero port
        const configured = environment["DESTACK_PORT"];
        const port = configured === undefined ? MACHINE_PORT : Number(configured);
        if (!/^\d+$/u.test(configured ?? String(MACHINE_PORT)) || port < 1 || port > 65535) {
            throw MachineError.of(
                "CONFIGURATION_BROKEN",
                `DESTACK_PORT is not a port: '${configured}'`,
            );
        }

        // open a tunnel to the relay at a URL, else to the account residency's
        const url = environment["DESTACK_RELAY"];
        if (url !== undefined && !URL.canParse(url)) {
            throw MachineError.of("CONFIGURATION_BROKEN", `DESTACK_RELAY is not a URL: '${url}'`);
        }
        const relay: ComputerOptions["relay"] =
            url === undefined ? { kind: "residency" } : { kind: "url", url };

        // ship the distribution the environment names, else the one in a release's resources
        const { executable, runtime } = host.process;
        const bundled = runtime.isStandalone
            ? join(Distribution.resources(executable), DISTRIBUTION_DIRECTORY)
            : undefined;
        const shipped = environment["DESTACK_DISTRIBUTION"] ?? bundled;
        if (shipped === undefined) {
            throw MachineError.of(
                "CONFIGURATION_BROKEN",
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
        bundle: Pick<ComputerOptions, "localObjects">,
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
            throw MachineError.of(
                "MACHINE_BLOCKED",
                "cannot start the Destack machine: it is already running",
            );
        }
        cleanup.defer(() => lock.close());

        // record the process's telemetry from its start: in its own log, and in its owner's home once enrolled
        let running: Computer | undefined;
        const telemetry = await Computer.#startTelemetry(environment, () => running?.machine);
        const instruments = TelemetryScope.of(MACHINE_PACKAGE, telemetry);

        // choose the keys' store, and log it
        const store = Computer.keychainStore(directory, environment);
        instruments.log.info("destack.keychain.chosen", { "destack.keychain.store": store.kind });
        const recorded = Environment.instrument(environment, telemetry);

        // compose the machine with the system's keychains, this channel's distribution and its toolchain
        const stopping = new AbortController();
        const home = UniverseHome.current(environment);
        const installed = home.distribution(environment);
        const { isStandalone } = environment.process.runtime;
        const toolchain = isStandalone
            ? () => installed.distribution.toolchain()
            : async () => Toolchain.workspace();
        const shipped = await BunDistribution.read(configuration.shipped, files);
        const started = await Computer.start({
            isolation: "sandbox",
            directory,
            environment: recorded,
            universe: configuration.universe,
            port,
            ...(configuration.relay === undefined ? {} : { relay: configuration.relay }),
            ...bundle,
            shipped,
            layer: await Computer.#layer(configuration.shipped, shipped, environment),
            keychain: (service) => new BunKeychain(service, files, store),
            process: {
                version: MACHINE_PACKAGE.version,
                pid: environment.process.id,
                startedAt: environment.clock.now(),
            },
            ...(isStandalone ? { distribution: installed } : {}),
            toolchain,
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
                    idleTimeout: Computer.idleTimeout,
                },
            ],
            environment,
            signal: stopping.signal,
            shutdown: () => stopping.abort(),
            close: async () => {
                // note the stop while the machine's telemetry still runs, then close everything
                note("destack.machine.stopped");
                await cleanup.disposeAsync();
            },
            ready: async ([address]) => {
                // require the listener's address
                if (address === undefined) {
                    throw MachineError.of(
                        "LISTENER_BROKEN",
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
                note("destack.machine.started", {
                    "process.pid": environment.process.id,
                    "server.port": listening,
                });
            },
        });
    }

    /** Serve one of a workload instance's packages through the gateway. */
    static #serving(instance: WorkloadInstance, handle: Package): Pick<Server, "fetch"> {
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

        // forward the telemetry of a process on this computer into the owner's home, as the machine's own
        if (isLoopback && url.pathname.startsWith(`${OriginPath.telemetry}/`)) {
            return Computer.#observe(this.machine, request);
        }
        // refuse a service of a partition this machine does not serve yet
        else if (own !== undefined && "error" in own) {
            return refusal(own.error, this.machine.environment.telemetry);
        }
        // serve one of the machine's own services
        else if (own?.found !== undefined) {
            return own.found.server.fetch(own.found.request);
        }
        // serve a name at any host but the machine's own: loopback, its endpoint and its loopback origins
        else if (!Computer.#isOwnHost(url.hostname, this.machine.endpoint)) {
            return this.machine.serveName(request);
        }

        // hand anything else to the spaces: their origins, services, workloads' calls and buckets
        return this.machine.serve(request);
    }

    /** Report whether a host is the computer's own: loopback, its endpoint, or its origins. */
    static #isOwnHost(hostname: string, endpoint: string): boolean {
        // take loopback, the endpoint, and the installation and S3 origins below the loopback domain
        const isLoopback = hostname === LOOPBACK_DOMAIN || HostAddress.isLoopback(hostname);
        const isEndpoint = hostname === new URL(endpoint).hostname;
        const isInstallation = InstallationOrigin.parse(hostname, LOOPBACK_DOMAIN) !== undefined;
        const isBucket = S3Origin.parse(hostname, LOOPBACK_DOMAIN) !== undefined;

        return isLoopback || isEndpoint || isInstallation || isBucket;
    }

    /** Route a request the relay forwards through this machine's tunnel. */
    async relay(request: Request): Promise<Response> {
        // serve a name the relay routes here, else the machine's peers
        const url = new URL(request.url);
        const { domains } = await this.machine.universe();
        if (MachineAddress.parse(url.hostname, domains.machine) === undefined) {
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

    /** Capture a failure the computer reports. */
    static capture(telemetry: Environment["telemetry"], event: string, error: unknown): void {
        TelemetryScope.of(MACHINE_PACKAGE, telemetry).captureException(error, {
            isEscaped: true,
            tags: { event },
        });
    }

    /** Send an OTLP export into the telemetry of the owner's home, as the machine bound to it. */
    static async #observe(machine: Machine, request: Request): Promise<Response> {
        // locate the owner's home and send as this machine bound to it, below the path
        const home = await machine.home();
        const { url } = await machine.directory.locate(home, machine.clock.now());
        const { pathname } = new URL(request.url);
        const send = machine.fetch(stackPackage.id, { partition: home });

        return send(new Request(`${SpaceMount.url(url, home)}${pathname}`, request));
    }

    /** Start a machine process's telemetry: its own log, and its owner's home once enrolled. */
    static async #startTelemetry(
        environment: Environment<"process">,
        machine: () => Machine | undefined,
    ): ReturnType<typeof startTelemetry> {
        // post each export to the telemetry of the owner's home where its machine answers, nothing before it enrolls
        const target = {
            endpoint: `${OWN_TELEMETRY_ORIGIN}${OriginPath.telemetry}`,
            fetch: (request: Request) =>
                Computer.#observe(present(machine(), "the enrolled machine"), request),
        };
        // write every record to the machine's own log too, beside its owner's home, and the export's own failures there alone
        const write = (line: string) => environment.process.stderr.writeSync(line);
        const options = LineExporter.options(environment.process.variables);
        const local = new LineExporter(write, options);
        const own = TelemetryScope.of(
            MACHINE_PACKAGE,
            local.telemetry(environment.clock, environment.telemetry.context),
        );
        const exporter = new OtlpExporter(
            async (signal, body, abort) => {
                const running = machine();
                if (running !== undefined && (await running.isEnrolled())) {
                    await OtlpExporter.post(target, signal, body, abort);
                }
            },
            (error) => own.captureException(error, { isEscaped: true }),
        );

        return startTelemetry(exporter.options(MACHINE_PACKAGE, { local }), environment);
    }

    /** Stop serving and close what the machine opened, last opened first. */
    [Symbol.asyncDispose](): Promise<void> {
        // stop in its span, timing the shutdown
        return this.machine.scope.span("destack.computer.stop", {}, () =>
            this.#stack.disposeAsync(),
        );
    }
}

/** One of the computer's partitions: its running workload and its side of the sockets. */
interface ComputerPartition {
    /** The partition's running workload. */
    readonly instance: WorkloadInstance;
    /** The partition's side of the sockets the machine's host keeps for it. */
    readonly hibernation: Hibernation;
}
