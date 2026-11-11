import type { Environment, OperatingSystem } from "@destack/environment";
import { principal, type TableMapping } from "@destack/access";
import { assignment, placement, Platform } from "@destack/account/object";
import type { Launcher } from "../../launcher/index.ts";
import {
    defineMount,
    type CallKey,
    type Follow,
    type MethodContextOf,
    ObjectError,
    type ObjectType,
    Package,
} from "@destack/object";
import { eq } from "@destack/db";
import type { TunnelStatus } from "@destack/tunnel";
import { PackageVersion } from "@destack/package";
import objectServer, {
    implementPackage,
    ObjectStore,
    type PackageHandler,
    type PackageImplementation,
} from "@destack/object/server";
import type { WorkloadContext } from "@destack/object/workload";
import type { EventStore } from "@destack/event";
import { DistributionHost, distributionServer } from "@destack/update/server";
import type { Device } from "../device/device.ts";
import type { NativeClient } from "@destack/account/native";
import { PEOPLE_PATH } from "../device/credential.ts";
import { type Identifier, present, schema } from "@destack/schema";
import { buildShape } from "@destack/stack/object";
import { RequestId, Scope, Subject } from "@destack/sync";
import { Machine } from "../../machine/index.ts";
import { MachineError } from "../../error/index.ts";
import { computer, type MachineEnrollment } from "../device/object/index.ts";
import { SignInRunner } from "./sign-in.ts";
import { ScopeReceiver } from "./scope.ts";
import type { BuildRunner } from "../build.ts";
import machinePackage from "../../package.ts";
import { CheckoutServer, PreviewSource } from "../checkout/server/index.ts";
import type {} from "@destack/package/import-meta";

/** The longest machine name taken from a computer's name: a DNS label (RFC 1035 2.3.4). */
const NAME_LENGTH = 63;

/** How long a lookup waits for the directory to sync after the machine enrolls: a minute. */
const DIRECTORY_SYNC_MILLISECONDS = 60_000;

/** The machine package's server on a computer: its record and the machine's own partition. */
export class ComputerServer {
    /** Authorize the machine's own store: the placements and assignments in its replica. */
    static readonly POLICIES: readonly (ObjectType | TableMapping)[] = [placement, assignment];

    /** This machine, enrolling and revoking itself at the universe. */
    readonly machine: Machine;
    /** Open the launcher of a person signed in on this computer. */
    readonly launcher: (login: Identifier<"login">) => Promise<Launcher>;
    /** Request graceful shutdown. */
    readonly shutdown: () => void;
    /** The computer the machine runs on: its system, architecture, release and name. */
    readonly platform: Environment<"host">["host"];
    /** The runner of the builds assigned to this machine. */
    readonly runner: BuildRunner;
    /** What the development workflow builds with. */
    readonly build: Pick<CheckoutServer, "connect" | "compilers" | "host">;
    /** This computer's device, whose partition stores the computer's record and distribution. */
    readonly device: Device<NativeClient>;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The installed distribution the machine stages and activates, none in development. */
    readonly distribution: DistributionHost | undefined;
    /** The events of the machine's own partition: its calls and its telemetry. */
    readonly events: EventStore;
    /** The events of the device's partition: the calls to its local objects. */
    readonly deviceEvents: EventStore;
    /** The device's local object types beside the computer's record. */
    readonly localObjects: readonly ObjectType[];
    /** The running process the computer's record describes. */
    readonly process: Parameters<typeof ComputerServer.record>[2];
    /** The enrollment the universe declares for a cloud machine, none for a person's machine. */
    readonly enrollment: MachineEnrollment | undefined;
    /** Receive each partition's store once it starts, as the computer waits for its device's. */
    readonly receive: {
        readonly device: (store: ObjectStore) => void;
        readonly machine: (store: ObjectStore) => void;
    };

    /** Serve the machine package on a computer. */
    constructor(
        fields: Pick<
            ComputerServer,
            | "machine"
            | "launcher"
            | "shutdown"
            | "platform"
            | "runner"
            | "build"
            | "device"
            | "callKey"
            | "distribution"
            | "events"
            | "deviceEvents"
            | "localObjects"
            | "process"
            | "enrollment"
            | "receive"
        >,
    ) {
        // set the machine, its verbs, its platform, its runner and its builds
        this.machine = fields.machine;
        this.launcher = fields.launcher;
        this.shutdown = fields.shutdown;
        this.platform = fields.platform;
        this.runner = fields.runner;
        this.build = fields.build;

        // set the device, the keys and events of both partitions, and what the record states
        this.device = fields.device;
        this.callKey = fields.callKey;
        this.distribution = fields.distribution;
        this.events = fields.events;
        this.deviceEvents = fields.deviceEvents;
        this.localObjects = fields.localObjects;
        this.process = fields.process;
        this.enrollment = fields.enrollment;
        this.receive = fields.receive;
    }

    /** Serve the device's partition: the computer's record, local objects and distribution. */
    async serveDevice(): Promise<{ readonly packages: readonly PackageHandler[] }> {
        // serve the device's own objects, the computer's record and the distribution over the device's store
        const { device } = this;
        const machine = this.machine.id;
        const own = this.localObjects.filter((type) => type.package.id === machinePackage.id);
        const others = this.localObjects.filter((type) => type.package.id !== machinePackage.id);
        const store = await device.store(
            [
                this.#implementDevice(own),
                ...device.servers(others),
                distributionServer(this.distribution),
                objectServer,
            ],
            this.callKey,
            this.deviceEvents,
        );

        // write the computer's record and an installed distribution's as the machine
        await ComputerServer.record(
            store,
            { device: device.id, machine },
            this.process,
            this.enrollment,
        );
        if (this.distribution !== undefined) {
            await DistributionHost.record(store, device.id, this.distribution);
        }

        // hand the started store to the process
        this.receive.device(store);

        return { packages: store.packages };
    }

    /** Serve the machine's own partition once it enrolls: checkouts, previews and replicas. */
    async serveMachine(
        context: WorkloadContext,
    ): Promise<{ readonly packages: readonly PackageHandler[] }> {
        // serve the machine's checkouts, previews and replicas over its store, home of its partition
        const machine = this.machine.id;

        // serve each package as the machine's own, acting as the machine
        const servers = [this.#implementMachine(), objectServer];

        // open the store, home of the machine's partition
        const store = new ObjectStore({
            servers,
            principal: principal.machine.reference(Scope.universe.id, machine),
            partitions: [machine],
            policies: ComputerServer.POLICIES,
            database: this.machine.database,
            blobs: this.machine.cache.store.blobs.storage,
            environment: context.environment,
            callKey: this.callKey,
            events: this.events,
            origin: { package: machinePackage, machineId: machine },
            machine,
            // follow the machine's chain above it once enrolled
            chain: () => this.machine.chain(),
            publisher: (scope) => this.machine.publisher(scope),
        });
        this.receive.machine(store);

        return { packages: store.packages };
    }

    /** Serve the computer's record in its device, beside this package's local objects. */
    #implementDevice(objects: readonly ObjectType[]): PackageImplementation {
        // declare the record beside the device's local types under this package
        const handle = Package.of(PackageVersion.of(machinePackage), {
            objects: [computer, ...objects],
            // forward the owner's calls as a person signed in here: /people/<login>/<target>
            mounts: [
                defineMount({ name: "people", path: PEOPLE_PATH, callers: [principal.device] }),
            ],
        });

        return implementPackage(handle, {
            objects: [this.#computer(), ...objects],
            mounts: { people: (request) => this.#people(request) },
            // serve the spaces as the machine while its record states an enrollment, and carry out sign-ins
            controllers: [
                () => this.machine.controller(),
                (store) => new SignInRunner(this.device, store).controller(),
            ],
        });
    }

    /** Serve the machine's partition once enrolled: checkouts, previews, replicas and builds. */
    #implementMachine(): PackageImplementation {
        // follow the machine's replicas and its builds, and serve its checkouts and previews
        const checkouts = new CheckoutServer({ ...this.build, machine: this.machine });
        const { id } = this.machine;
        const follows: readonly Follow[] = [
            // follow the directory, the identities and this machine's placements
            ...Machine.follows(id),
            // follow the builds this machine runs in each space it serves, and in each other space it has work in
            buildShape.per(placement, {
                where: { machineId: id },
                key: async (row) => ({
                    key: { scope: row.spaceId, runner: ComputerServer.#runner(id) },
                }),
            }),
            buildShape.per(assignment, {
                where: { machineId: id },
                key: async (row, view) => {
                    // follow the space of the placement the work is in
                    const placed = await view
                        .read(placement.table)
                        .findFirst({ where: { id: row.placementId } });

                    return placed === undefined
                        ? undefined
                        : { key: { scope: placed.spaceId, runner: ComputerServer.#runner(id) } };
                },
            }),
        ];

        return implementPackage(machinePackage, {
            objects: [checkouts.checkout(), checkouts.preview()],
            follows,
            // receive the machine's scope, run the builds assigned here and the checkouts' previews and sources
            controllers: [
                (store) => ScopeReceiver.controller(store, id),
                () => this.runner.controller(),
                (store) => new PreviewSource(store, checkouts).controller(),
            ],
        });
    }

    /** Answer a call below `/people/<login>` as that person, forwarding all but its own paths. */
    async #people(request: Request): Promise<Response> {
        // read the login and the target below the mount's path
        const { pathname } = new URL(request.url);
        const [, segment, ...target] = pathname.split("/");
        const loginId = schema.identifier("login").safeParse(segment);

        // forward as that person, refusing a path without a login
        if (!loginId.success) {
            throw MachineError.of("TARGET_INVALID", `'${pathname}' is invalid: it has no login`);
        }
        const login = await this.device.signedIn(loginId.data);
        const [kind] = target;

        // answer the person's account token for this computer's programs
        if (kind === "token") {
            return Response.json({ token: await login.user.accountToken() });
        }
        // answer a proof of the person's client
        else if (kind === "proof") {
            return Response.json({ proof: await login.prove() });
        }
        // forget the person signed out by a program of this computer
        else if (kind === "forget" && request.method === "POST") {
            await this.device.credential.forget(login.row);

            return Response.json({});
        }
        // forward anything else to its target
        else {
            return (await this.device.launcher(loginId.data)).forward(request, target);
        }
    }

    /** Record the running machine process on this computer's record, as the device storing it. */
    static async record(
        objects: Pick<ObjectStore, "database" | "mutate" | "clock" | "as" | "environment">,
        computerOf: {
            readonly device: Identifier<"device">;
            readonly machine: Identifier<"machine">;
        },
        process: { readonly version: string; readonly pid: number; readonly startedAt: number },
        enrollment: MachineEnrollment | undefined,
    ): Promise<void> {
        // record the process without a tunnel yet, and a declared enrollment, as the device
        const started = { ...process, tunnel: "none" as const };
        const recorded = enrollment === undefined ? started : { ...started, enrollment };
        const [row] = await objects.database
            .select()
            .from(computer.table)
            .where(eq(computer.table.machineId, computerOf.machine));
        const now = objects.clock.now();
        const context = objects.as(computer, now);

        // start the stored record, else create it in the device
        const call =
            row === undefined
                ? computer.call("create", {
                      ...computer.in(computerOf.device),
                      machineId: computerOf.machine,
                      ...recorded,
                  })
                : computer.call("start", { ...computer.target(row), ...recorded });
        await objects.mutate(context, { id: RequestId.create(objects.environment), calls: [call] });
    }

    /** Record whether the machine process's relay tunnel is open or dialing, as the device. */
    static async link(
        objects: Pick<ObjectStore, "database" | "mutate" | "clock" | "as" | "environment">,
        machine: Identifier<"machine">,
        tunnel: TunnelStatus,
    ): Promise<void> {
        // update the record started already
        const [row] = await objects.database
            .select()
            .from(computer.table)
            .where(eq(computer.table.machineId, machine));
        const recorded = present(row, `the record of machine ${machine}`);
        const call = computer.call("link", { ...computer.target(recorded), tunnel });
        const context = objects.as(computer, objects.clock.now());
        await objects.mutate(context, { id: RequestId.create(objects.environment), calls: [call] });
    }

    /** Read a computer's operating system as machines record it, refusing other systems. */
    static platform(os: OperatingSystem): Platform {
        const name = Platform.ofNode(os);
        if (name === undefined) {
            throw MachineError.of(
                "CONFIGURATION_BROKEN",
                `the machine does not run on '${os}' computers`,
            );
        }

        return name;
    }

    /** Implement the computer's record: enroll, revoke, locate spaces, and stop. */
    #computer() {
        return computer.implement({
            methods: {
                locate: async (call) => {
                    // read the directory once it is a complete snapshot, which follows the enrollment
                    const { machine } = this;
                    const timeout = machine.clock.timeout(DIRECTORY_SYNC_MILLISECONDS);
                    if (!(await machine.synced(timeout))) {
                        throw ObjectError.of(
                            "REPLICA_UNAVAILABLE",
                            "the machine's directory is unavailable: it has no complete snapshot yet",
                        );
                    }

                    // answer where the space's machine answers, this machine's origin for its own
                    return machine.locate(call.input.space);
                },
                resolve: async (call) => {
                    // find the account claiming the handle in the directory replica
                    const { handle } = call.input;
                    const id = await this.machine.directory.account(handle);
                    if (id === undefined) {
                        throw MachineError.of(
                            "ACCOUNT_NOT_FOUND",
                            `account @'${handle}' not found`,
                        );
                    }

                    return { id };
                },
                stop: async (call) => {
                    // stop once the answer is sent
                    void call.environment.clock.sleep(0).then(() => this.shutdown());

                    return call.target;
                },
                enroll: {
                    prepare: (context) => this.#enroll(context),
                    handle: (context) => context.update({ enrollment: context.prepared }),
                    settle: async (_call, settled) => {
                        // withdraw an enrollment this machine failed to complete
                        if (settled.status === "rolledBack" && settled.prepared !== undefined) {
                            await this.machine.revoke(settled.prepared);
                        }
                    },
                },
                revoke: {
                    prepare: async (context) => {
                        // require a valid enrollment
                        const { enrollment } = context.target;
                        if (enrollment?.status !== "enrolled") {
                            throw MachineError.unenrolled("revoking its enrollment");
                        }

                        return enrollment;
                    },
                    handle: (context) =>
                        context.update({
                            enrollment: { ...context.prepared, status: "revoked" },
                        }),
                    settle: async (_call, settled) => {
                        // revoke the machine at its issuer once revoked here
                        if (settled.status === "committed") {
                            await this.machine.revoke(settled.prepared);
                        }
                    },
                },
            },
        });
    }

    /** Enroll this machine with the universe as the approving person. */
    async #enroll(context: MethodContextOf<typeof computer, "enroll", unknown>) {
        // refuse enrolling twice and enrolling a revoked identity again
        const target = context.target;
        // refuse an enrolled machine
        if (target.enrollment?.status === "enrolled") {
            throw MachineError.of(
                "ENROLLMENT_BLOCKED",
                "cannot enroll the machine: it is already enrolled",
            );
        }
        // refuse a revoked identity
        else if (target.enrollment?.status === "revoked") {
            throw MachineError.of(
                "MACHINE_BLOCKED",
                "cannot enroll this machine: it was revoked, and a revoked machine can only enroll as a new machine",
            );
        }

        // enroll as the approving person at their issuer
        const { input } = context;
        const { session, user } = await this.launcher(input.login);

        return this.machine.enroll(session.accounts(), new URL(user.issuer.url).origin, {
            accountId: schema.identifier("account").parse(input.accountId),
            name: input.name ?? ComputerServer.#label(this.platform.hostname),
            version: target.version,
            platform: ComputerServer.platform(this.platform.os),
            architecture: this.platform.architecture,
            model: this.platform.hostname,
        });
    }

    /** Make a computer's name into a machine name of 63 characters at most. */
    static #label(name: string): string {
        // leave lowercase letters and digits and turn other runs into one hyphen
        return name
            .toLowerCase()
            .replace(/[^a-z0-9]+/gu, "-")
            .slice(0, NAME_LENGTH)
            .replace(/^-+|-+$/gu, "");
    }

    /** Key a machine as the runner of the builds it follows. */
    static #runner(machineId: Identifier<"machine">): string {
        return Subject.key(principal.machine.reference(Scope.universe.id, machineId));
    }
}
