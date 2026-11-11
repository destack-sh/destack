import { Failure } from "@destack/error";
import { RESOURCE_KIND } from "@destack/resource";
import { log, LogData } from "@destack/observability/object";
import { BunWorkload } from "@destack/stack/bun";
import { MemoryBuild } from "@destack/package/test";
import { eq, Snapshot } from "@destack/db";
import settingPackage from "@destack/setting/package";
import * as settingObject from "@destack/setting/object";
import * as accountObject from "@destack/account/object";
import type { Login } from "@destack/account/native";
import { BuildStore } from "@destack/build/store";
import { placement } from "@destack/account/object";
import type { Crypto, Environment } from "@destack/environment";
import { type DependencyResolution, Package } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { fixtureOutputs, LocalRuntime } from "@destack/stack/test";
import { found, Identifier, present } from "@destack/schema";
import {
    definePackage,
    Egress,
    InstallationMount,
    PackageMount,
    SpaceMount,
    CallError,
    defineInstallationBinding,
    installation,
} from "@destack/object";
import { DOMAINS, UniverseOrigin } from "@destack/account/address";
import accountPackage from "@destack/account/package";
import { createClient } from "@destack/object/client";
import { RequestId, Scope, ObjectReference } from "@destack/sync";
import { defineWorkload, type RunnerOptions } from "@destack/object/workload";
import { BucketError } from "@destack/bucket";
import { bucket } from "@destack/stack/object";
import * as stackObject from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import objectPackage from "@destack/object/package";
import { StepUp, principal } from "@destack/access";
import { expect, refusal, test } from "@destack/test";
import boardPackage from "../../tests/fixture/board/src/package.ts";
import machinePackage from "../package.ts";
import type { TestRelay } from "@destack/relay/test";
import {
    OWNER_EMAIL,
    PUBLISHER,
    RELAY_NODE,
    TestComputer,
    type TestComputerOptions,
    type TestWorld,
    SCENARIO_MILLISECONDS,
    TestDistribution,
} from "./test/index.ts";

/** The step-up a move of a space asks for: a second factor within the last 15 minutes. */
const MOVE_STEP_UP = StepUp.schema.parse({ assurance: 2, maxAge: 900_000 });

/** The package both spaces install: one workload answering each call with its caller. */
const echo = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000a4",
    name: "@acme/echo",
    version: "2026.10.0-dev.1",
});

/** The echo package's handle. */
const echoPackage = definePackage({}, { package: echo });

/** The echo package's workload, which answers each call with its path and caller. */
const echoWorkload = defineWorkload(
    {
        name: "main",
        start: (context) => ({
            packages: [
                {
                    package: echoPackage,
                    router: {},
                    environment: context.environment,
                    fetch: async (request, called) =>
                        Response.json({
                            path: new URL(request.url).pathname,
                            caller: called.caller().subject.id,
                            scope: called.authentication?.credential.scope ?? null,
                        }),
                },
            ],
        }),
    },
    { package: echo },
);

/** The package that calls an echo installation of another space. */
const visitor = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000a5",
    name: "@acme/visitor",
    version: "2026.10.0-dev.1",
});

/** The visitor's binding of an echo installation, which a space binds across spaces. */
const peerBinding = defineInstallationBinding("peer", echoPackage, {}, { package: visitor });

/** The visitor's workload, which serves nothing and calls its peer through its machine's egress. */
const visitorWorkload = defineWorkload(
    { name: "main", start: () => ({ packages: [] }) },
    { package: visitor },
);

/** Write a package's build in memory. */
async function workloadBuild(
    built: Package,
    declarations: readonly MemoryDeclaration[],
    dependencies: readonly BuildReader[],
    crypto: Crypto,
): Promise<BuildReader> {
    return MemoryBuild.of(
        built,
        declarations.map((declaration) => ({
            ...RESOURCE_KIND.identity,
            name: declaration.name,
            description: declaration.description,
        })),
        crypto,
        {
            dependencies: await releases(dependencies, crypto),
            outputs: fixtureOutputs({
                server: {
                    runtime: "bun",
                    emit: true,
                    workloads: {
                        main: {
                            entrypoint: ".",
                            packages: [],
                            resources: declarations.map((declaration) => ({
                                packageId: built.id,
                                name: declaration.name,
                            })),
                            secrets: [],
                            compute: {},
                            sandbox: {},
                        },
                    },
                    views: {},
                },
            }),
        },
    );
}

/** Resolve builds as the Destack releases a build depends on, by package name. */
async function releases(
    dependencies: readonly BuildReader[],
    crypto: Crypto,
): Promise<Record<string, DependencyResolution>> {
    const resolved: Record<string, DependencyResolution> = {};
    for (const dependency of dependencies) {
        resolved[PackageManifest.package(dependency.manifest).name] = {
            kind: "destack",
            package: PackageManifest.package(dependency.manifest),
            manifest: await BuildStore.digest(dependency.manifest, crypto),
        };
    }

    return resolved;
}

/** A resource a build in memory declares: its name and description. */
interface MemoryDeclaration {
    /** The declaration's name. */
    readonly name: string;
    /** The declaration's description. */
    readonly description: Readonly<Record<string, unknown>>;
}

/** The echo build, declaring nothing beside its workload. */
function echoBuild(crypto: Crypto): Promise<BuildReader> {
    return workloadBuild(echo, [], [], crypto);
}

/** The visitor build, declaring its peer binding of the echo package at an echo release. */
function visitorBuild(echoRelease: BuildReader, crypto: Crypto): Promise<BuildReader> {
    const { name, kind, spec } = peerBinding;
    const declared = [{ name, description: { name, kind, spec } }];

    return workloadBuild(visitor, declared, [echoRelease], crypto);
}

/** Run the echo scenarios' workloads in the test's own process, on a node's clock and network. */
function workloads(environment: Environment): ReadonlyMap<string, RunnerOptions> {
    return new Map([
        [echo.id, new BunWorkload(echoWorkload, {}).runner(environment)],
        [visitor.id, new BunWorkload(visitorWorkload, { peer: peerBinding }).runner(environment)],
    ]);
}

/** A computer of the owner's serving a space, with the owner signed in on its device. */
interface Served {
    /** The computer. */
    readonly computer: TestComputer;
    /** The owner's login on the computer's device. */
    readonly login: Login["row"];
    /** The space the computer serves. */
    readonly space: Identifier<"space">;
}

/** A served computer running its instances in the test's process. */
interface Echoing extends Served {
    /** The runtime running the computer's instances. */
    readonly runtime: LocalRuntime;
    /** The failures the runtime reported. */
    readonly failures: unknown[];
    /** The node the computer runs as on the simulated network. */
    readonly node: string;
}

/** Start a computer, enroll it in an account as the owner and serve a new space on it. */
async function openComputer(
    world: TestWorld,
    accountId: Identifier<"account">,
    name: string,
    options: TestComputerOptions = {},
): Promise<Served> {
    // start the computer and sign the owner in on its device
    const computer = await TestComputer.open({ ...options, world });
    const login = await computer.signIn(OWNER_EMAIL);

    // enroll it in the account and create the space on it
    await computer.enroll(login, accountId);
    const space = await computer.space(login, accountId, name);

    return { computer, login, space };
}

/** Serve a space on a computer of the owner's account through a relay, running echo in process. */
async function openEcho(world: TestWorld, name: string, relay: TestRelay): Promise<Echoing> {
    // run instances in the test's process with their packages' workloads, on the computer's node
    const node = `computer-${name}`;
    const runners = workloads(world.simulation.environment(node));
    const runtimes: LocalRuntime[] = [];
    const failures: unknown[] = [];
    const served = await openComputer(world, world.owner.account, name, {
        node,
        relay: relay.url,
        runtime: (egress, identity) => {
            const runtime = new LocalRuntime({
                scheduler: world.simulation.scheduler,
                egress,
                identity,
                runner: (spec) => found(runners, PackageManifest.package(spec.build.manifest).id),
                report: (error) => failures.push(error),
            });
            runtimes.push(runtime);

            return runtime;
        },
    });

    return { ...served, runtime: present(runtimes[0], "the computer's runtime"), failures, node };
}

/** Connect to the object package of a computer's space as the owner signed in on its device. */
async function objectsOf(served: Served) {
    const { device } = served.computer;

    return createClient(
        objectPackage,
        (await device.launcher(served.login.id)).endpoint(
            objectPackage.id,
            await device.locate(served.space),
        ),
        device.environment,
    );
}

/** Provision a bucket of a definition in a computer's space. */
async function keepBucket(
    served: Served,
    definition: {
        readonly name: string;
        readonly spec: Record<string, never>;
        readonly package: Pick<Package, "id" | "version">;
    },
): Promise<Identifier<"bucket">> {
    // request the bucket in the space, as the space's stack declares it
    const { computer, space } = served;
    const database = (await computer.machine.space(space)).database;
    const id = Identifier.create("bucket", computer.environment);
    const now = computer.clock.now();
    await database.transaction(async (transaction) => {
        await transaction.insert(bucket.table).values({
            id,
            scope: space,
            ...ObjectReference.columns(
                "owner",
                principal.space.reference(Scope.universe.id, space),
            ),
            name: definition.name,
            definitionPackageId: definition.package.id,
            definitionVersion: definition.package.version,
            definitionName: definition.name,
            spec: definition.spec,
            createdAt: now,
            updatedAt: now,
        });
    });

    // wait for the machine to provision it
    await computer.waitUntil(database, async () => {
        const [row] = await database
            .select({ reference: bucket.table.reference })
            .from(bucket.table)
            .where(eq(bucket.table.id, id));

        return row !== undefined && row.reference !== null;
    });

    return id;
}

/** Install a published echo release in a computer's space, bound to a peer, until it runs. */
async function install(served: Echoing, release: BuildReader, name: string, peer?: Peer) {
    // install the release as the owner
    const { computer, space } = served;
    const released = PackageManifest.package(release.manifest);
    const service = await objectsOf(served);
    const created = await service.installation.create({
        scope: space,
        requestId: RequestId.create(computer.environment),
        packageId: released.id,
        selection: { kind: "release", version: released.version },
        name,
    });

    // bind its peer declaration and wait for the instances of its workloads
    if (peer !== undefined) {
        await bind(served, created.id, peer);
    }
    const workloadNames = Object.keys(release.manifest.outputs["server"]?.workloads ?? {});
    await serving(served, created.id, workloadNames.length);

    return { id: created.id, name };
}

/** Wait until a computer's space store serves an installed package's objects. */
async function storing(served: Served, packageId: string): Promise<void> {
    const database = (await served.computer.machine.space(served.space)).database;
    await served.computer.waitUntil(
        database,
        async () => (await storePackage(served, packageId)) !== undefined,
    );
}

/** Find the package a computer's space store serves for an installed package. */
async function storePackage(served: Served, packageId: string) {
    const { instance } = await served.computer.machine.space(served.space);

    return instance.packages.find((handle) => handle.id === packageId);
}

/** Wait until a computer runs the instances of an installation's workloads, one by default. */
async function serving(
    served: Echoing,
    installationId: Identifier<"installation">,
    workloadCount = 1,
    spaceId: Identifier<"space"> = served.space,
): Promise<void> {
    const database = (await served.computer.machine.space(spaceId)).database;
    await served.computer
        .waitUntil(
            database,
            async () =>
                (await stackObject.Instance.endpoints(database, installationId)).filter(
                    (endpoint) => served.runtime.isRunning(endpoint.instanceId),
                ).length === workloadCount,
        )
        .catch(async (error: unknown) => {
            // name the installation's ready condition beside the wait's failure
            const [row] = await database
                .select({ conditions: installation.table.conditions })
                .from(installation.table)
                .where(eq(installation.table.id, installationId));
            const ready = row?.conditions["Ready"];
            throw new Error(
                `${installationId} runs no instance: ${ready?.reason} ${ready?.message}`,
                {
                    cause: error,
                },
            );
        });
}

/** Serve a board's space of a shared account on a computer in the home's world. */
async function openBoard(
    home: TestComputer,
    account: Identifier<"account">,
    options: TestComputerOptions = {},
): Promise<Served> {
    // serve the board's space of the shared account on a computer enrolled in it
    const { world } = home;
    const served = await openComputer(world, account, "board", options);

    // publish the board as the platform's operator, and install it
    const { computer, login, space } = served;
    const database = (await computer.machine.space(space)).database;
    const build = await TestDistribution.fixture("board");
    await home.publish(build);
    await computer.install(login.id, space, build, "board");
    await storing(served, boardPackage.id);

    // wait until the board's sandboxed instance serves
    const board = await Snapshot.live(database)
        .read(installation.table)
        .findFirst({ where: { scope: space, packageId: boardPackage.id } });
    const boardId = present(board, "the board's installation").id;
    await computer.waitUntil(
        database,
        async () => (await stackObject.Instance.endpoints(database, boardId)).length > 0,
    );

    return served;
}

/** Read the buckets a computer stores for a space, its own space's by default. */
async function buckets(served: Served, space: Identifier<"space"> = served.space) {
    return (await served.computer.machine.space(space)).buckets;
}

/** The installation of another space an echo installation calls as its peer. */
interface Peer {
    /** The called installation's name in its space. */
    readonly target: string;
    /** The space of the called installation. */
    readonly targetSpaceId: Identifier<"space">;
}

/** Bind an installation's peer declaration to an installation of another space, as a stack does. */
async function bind(
    served: Served,
    installationId: Identifier<"installation">,
    peer: Peer,
): Promise<void> {
    const { computer, space } = served;
    const database = (await computer.machine.space(space)).database;

    // select the peer for the visitor's binding, moving the generation as an update does
    const [row] = await database
        .select({
            bindings: installation.table.bindings,
            generation: installation.table.generation,
        })
        .from(installation.table)
        .where(eq(installation.table.id, installationId));
    const { bindings, generation } = present(row, `installation ${installationId}`);
    const target = { type: "installation", name: peer.target, spaceId: peer.targetSpaceId };
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({
                    bindings: {
                        ...bindings,
                        [visitor.id]: { ...bindings[visitor.id], [peerBinding.name]: { target } },
                    },
                    generation: generation + 1,
                })
                .where(eq(installation.table.id, installationId)),
    );
}

/** Connect to the stack package of a space at a computer's endpoint with an access token. */
function stackAt(served: Served, spaceId: Identifier<"space">, token: string) {
    const { computer } = served;

    return createClient(
        stackPackage,
        {
            url: `${SpaceMount.url(computer.endpoint, spaceId)}${PackageMount.path(stackPackage.id)}`,
            headers: { authorization: `Bearer ${token}` },
        },
        computer.device.environment,
    );
}

/** Connect to the stack package of a computer's space as the owner after a second factor. */
async function steppedUp(served: Served) {
    const { computer, space } = served;
    const elevated = await computer.elevate(MOVE_STEP_UP);
    const { user } = await computer.device.launcher(elevated.id);

    return stackAt(
        served,
        space,
        await user.token(stackPackage.id, { partition: space }, MOVE_STEP_UP),
    );
}

/** Serve a source space and a caller space on two of the owner's computers through a relay. */
async function openComputers() {
    // serve the owner's home and acme organization on the residency's cloud computer
    const { home } = await TestComputer.home({ organizations: [PUBLISHER] });
    const { world } = home;

    // serve both spaces
    const relay = await TestComputer.relay(world);
    const source = await openEcho(world, "source", relay);
    const caller = await openEcho(world, "caller", relay);

    // publish the echo package and the visitor as the owner of their acme organization
    const { crypto } = world.simulation.environment("builds");
    const echoRelease = await echoBuild(crypto);
    const visitorRelease = await visitorBuild(echoRelease, crypto);
    for (const release of [echoRelease, visitorRelease]) {
        await home.publish(release, world.owner);
    }

    // run the echo package in the source and the visitor in the caller
    const called = await install(source, echoRelease, "echo");
    const calling = await install(caller, visitorRelease, "visitor", {
        target: called.name,
        targetSpaceId: source.space,
    });

    // call the source's echo service from the caller's installation
    const call = async (path: string) => {
        const response = await caller.runtime.request(
            calling.id,
            Egress.address(called.name, source.space),
            `${PackageMount.path(echo.id)}${path}`,
            { method: "POST" },
        );

        return [response.status, await response.json()];
    };

    return { world, source, caller, called, calling, call };
}

test(
    "call an installation of a space on another computer through the relay as an installation of a space here, verified by the calling space's token",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // call the source's echo installation from the caller's installation
        const { source, calling, call } = await openComputers();
        const answered = await call("/greeting");

        // answer as the source space, seeing the calling installation
        expect(answered).toEqual([
            200,
            { path: "/greeting", caller: calling.id, scope: source.space },
        ]);
    },
);

test(
    "move a space with its app, database, builds and files to its caller's computer, answering its calls throughout, after cancelling a stuck move that leaves it serving where it was",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // store a file in another bucket of the source space and answer a first call
        const { world, source, caller, called, calling, call } = await openComputers();
        const files = await keepBucket(source, { name: "files", spec: {}, package: echo });
        await (await (await buckets(source)).open({ bucketId: files })).put("notes/a.txt", "first");
        const first = await call("/first");

        // cut the source computer off the relay, so its peers neither call nor capture it
        const { network } = world.simulation;
        network.partition(source.node, RELAY_NODE);

        // move the space as its owner, stepped up to a second factor, which the caller cannot capture
        const owner = await steppedUp(source);
        const target = caller.computer.machine.id;
        const stuck = await owner.transfer.create({
            spaceId: source.space,
            requestId: RequestId.create(world.universe.environment),
            targetId: target,
        });
        const moving = (await source.computer.machine.space(source.space)).database;
        const whileCut = await call("/cut");

        // cancel the stuck move as the owner
        await owner.transfer.cancel({
            spaceId: source.space,
            id: stuck.id,
            requestId: RequestId.create(world.universe.environment),
        });
        await serving(source, called.id);
        const [ended] = await moving
            .select({ outcome: stackObject.transfer.table.outcome })
            .from(stackObject.transfer.table)
            .where(eq(stackObject.transfer.table.id, stuck.id));

        // drop the caller's computer's replica of the cancelled move with the buckets it provisioned
        const replica = (await caller.computer.machine.space(source.space)).database;
        await caller.computer.waitUntil(
            replica,
            async () =>
                (
                    await replica
                        .select({ scope: Scope.table.scope })
                        .from(Scope.table)
                        .where(eq(Scope.table.scope, source.space))
                ).length === 0,
        );

        // drop the bucket once the cancelled move's restore in flight fails
        const abandoned = async () =>
            (await buckets(caller, source.space))
                .open({ bucketId: files })
                .then(() => "open")
                .catch((error: unknown) => (BucketError.is(error) ? error.reason : error));
        await caller.computer.waitUntil(
            replica,
            async () => (await abandoned()) === "NO_SUCH_BUCKET",
        );

        // rejoin the source computer to the relay, waiting for its tunnel
        const universe = world.universe.storage.database;
        const published = async () => {
            const [row] = await universe
                .select({ updatedAt: accountObject.endpoint.table.updatedAt })
                .from(accountObject.endpoint.table)
                .where(eq(accountObject.endpoint.table.parentId, source.computer.machine.id));

            return row?.updatedAt;
        };
        const before = await published();
        network.heal(source.node, RELAY_NODE);
        await source.computer.waitUntil(universe, async () => (await published()) !== before);
        const cancelled = await call("/cancelled");

        // move it again once the caller can capture it
        await owner.transfer.create({
            spaceId: source.space,
            requestId: RequestId.create(world.universe.environment),
            targetId: target,
        });
        await source.computer.waitUntil(
            universe,
            async () =>
                (
                    await universe
                        .select({ machine: placement.table.machineId })
                        .from(placement.table)
                        .where(eq(placement.table.spaceId, source.space))
                )[0]?.machine === target,
        );
        await serving(caller, called.id, 1, source.space);
        const moved = await call("/moved");

        // read the moved file on the caller's computer
        const stored = await (
            await (await buckets(caller, source.space)).open({ bucketId: files })
        ).get("notes/a.txt");
        expect({
            first,
            whileCut,
            ended,
            cancelled,
            moved,
            file: await stored?.text(),
            reports: [
                ...source.computer.reports,
                ...source.failures,
                ...caller.computer.reports,
                ...caller.failures,
            ],
        }).toEqual({
            first: [200, { path: "/first", caller: calling.id, scope: source.space }],
            whileCut: [
                503,
                {
                    defined: true,
                    code: "SERVICE_UNAVAILABLE",
                    status: 503,
                    message: "the machine is disconnected",
                    reason: "MACHINE_DISCONNECTED",
                    domain: "@destack/relay",
                },
            ],
            ended: { outcome: "cancelled" },
            cancelled: [200, { path: "/cancelled", caller: calling.id, scope: source.space }],
            moved: [200, { path: "/moved", caller: calling.id, scope: source.space }],
            file: "first",
            reports: [],
        });
    },
);

test(
    "relay a sandboxed instance's telemetry through its machine into its space's observability, stamped with the installation, instance and build its secret proves",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // run the board sandboxed
        const { home, organizations } = await TestComputer.home({ organizations: [PUBLISHER] });
        const source = await openBoard(home, found(organizations, PUBLISHER.handle));
        const { computer, space } = source;
        const sourced = (await computer.machine.space(space)).database;
        const [boards] = await Snapshot.live(sourced)
            .read(installation.table)
            .findMany({ where: { scope: space, packageId: boardPackage.id } });
        const boardId = present(boards, "the board's installation").id;
        const [running] = await stackObject.Instance.endpoints(sourced, boardId);
        const [built] = await sourced
            .select({ manifest: stackObject.build.table.manifest })
            .from(stackObject.rollout.table)
            .innerJoin(
                installation.table,
                eq(installation.table.id, stackObject.rollout.table.installationId),
            )
            .innerJoin(
                stackObject.installationRevision.table,
                eq(stackObject.installationRevision.table.id, installation.table.revisionId),
            )
            .innerJoin(
                stackObject.build.table,
                eq(stackObject.build.table.id, stackObject.installationRevision.table.buildId),
            )
            .where(eq(stackObject.rollout.table.installationId, boardId));
        const instance = present(running, "the board's instance").instanceId;
        const manifest = present(built?.manifest, "the board's build manifest");

        // read the started records the board's runner exported to its machine
        const { events } = (await computer.machine.space(space)).volume;
        const started = async () =>
            (
                await events.query(
                    log.eventKind(),
                    { scopes: [space], where: { installationId: boardId } },
                    { limit: 10 },
                )
            ).events
                .filter((entry) => entry.keys.name === "workload.started")
                .map((entry) => [
                    entry.keys.installationId,
                    LogData.parse(entry.data).instance,
                    entry.keys.build,
                ]);

        // record it under the board's installation, its running instance and its build
        await computer.waitUntil(sourced, async () => (await started()).length > 0);
        expect([await started(), computer.reports]).toEqual([[[boardId, instance, manifest]], []]);
    },
);

test(
    "accept a peer machine's universe token only for the space it is bound to, fence only a space it receives, and refuse it at once after its key is revoked",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // serve two spaces and enroll a peer computer through the relay
        const world = await TestComputer.world();
        const relay = await TestComputer.relay(world);
        const served = await openComputer(world, world.owner.account, "work", { relay: relay.url });
        const { computer, space } = served;
        const other = await computer.space(served.login, world.owner.account, "other");
        const peer = await TestComputer.open({ world, relay: relay.url });
        await peer.enroll(await peer.signIn(OWNER_EMAIL), world.owner.account);

        // move the moving space to the peer as the owner, stepped up to a second factor
        const moving = await (
            await steppedUp(served)
        ).transfer.create({
            spaceId: space,
            requestId: RequestId.create(computer.environment),
            targetId: peer.machine.id,
        });
        expect([
            (await computer.device.locate(space)).placement.machine,
            await world.universe.storage.database
                .select({ target: placement.table.targetId })
                .from(placement.table)
                .where(eq(placement.table.spaceId, space)),
        ]).toEqual([computer.machine.id, [{ target: peer.machine.id }]]);

        // grant the peer tokens in the universe, the moving space and the other space it does not receive
        const issuer = UniverseOrigin.issuer(world.universe.origin);
        const grant = (binding: { readonly partition?: Identifier<"space"> }) =>
            peer.machine.identity
                .at(issuer, (request) => peer.device.environment.network.fetch(request))
                .token(stackPackage.id, binding);
        const unbound = await grant({});
        const bound = await grant({ partition: space });
        const elsewhere = await grant({ partition: other });

        // fence the moving space with the bound token alone and refuse it once the peer's key is revoked
        const peerKeys = async () =>
            await computer.machine.database
                .select({ revokedAt: accountObject.key.table.revokedAt })
                .from(accountObject.key.table)
                .where(eq(accountObject.key.table.parentId, peer.machine.id));
        await computer.waitUntil(computer.machine.database, async () => {
            const keys = await peerKeys();

            return keys.length === 1 && keys.every((each) => each.revokedAt === null);
        });
        const fence = (spaceId: Identifier<"space">, token: string) =>
            stackAt(served, spaceId, token).transfer.fence({
                spaceId,
                id: moving.id,
                requestId: RequestId.create(computer.environment),
            });
        const outcomes = [
            await refusal(fence(space, unbound)),
            await refusal(fence(other, bound)),
            await refusal(fence(other, elsewhere)),
            await refusal(fence(space, bound)),
        ];
        await peer.connect(machinePackage).computer.revoke({
            deviceId: peer.device.id,
            id: peer.machine.id,
            requestId: RequestId.create(computer.environment),
        });
        await computer.waitUntil(computer.machine.database, async () =>
            (await peerKeys()).every((each) => each.revokedAt !== null),
        );
        expect({
            outcomes,
            revoked: await refusal(fence(space, bound)),
        }).toEqual({
            outcomes: [
                [
                    "UNAUTHORIZED",
                    "TOKEN_UNBOUND",
                    "universe tokens reach this machine bound to a partition",
                ],
                [
                    "FORBIDDEN",
                    "FOREIGN_PARTITION_TOKEN",
                    "the universe token is bound to another partition",
                ],
                ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${other} not found`],
                "done",
            ],
            revoked: ["UNAUTHORIZED", "KEY_REVOKED", "the machine token's key is no longer valid"],
        });
    },
);

test(
    "move a space through its machine as its owner once a stepped-up session exchanges for the universe's token of the space",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // serve a space and enroll another computer of the account to move it to
        const world = await TestComputer.world();
        const served = await openComputer(world, world.owner.account, "work");
        const { computer, space, login } = served;
        const peer = await TestComputer.open({ world });
        await peer.enroll(await peer.signIn(OWNER_EMAIL), world.owner.account);
        const move = (token: string) =>
            stackAt(served, space, token).transfer.create({
                spaceId: space,
                requestId: RequestId.create(computer.environment),
                targetId: peer.machine.id,
            });

        // challenge the person's token for a second factor
        const { user } = await computer.device.launcher(login.id);
        const challenge = await move(await user.token(stackPackage.id, { partition: space })).then(
            () => undefined,
            (error: unknown) => {
                const known = CallError.of(error);

                return known === undefined ? error : [known.code, known.data];
            },
        );

        // exchange the token again after a second factor, then move the space
        const stepUp = StepUp.schema.parse(Array.isArray(challenge) ? challenge[1] : undefined);
        const elevated = await computer.device.launcher((await computer.elevate(stepUp)).id);
        const moved = await move(
            await elevated.user.token(stackPackage.id, { partition: space }, stepUp),
        );
        expect({ challenge, moved: [moved.sourceId, moved.targetId] }).toEqual({
            challenge: ["UNAUTHORIZED", { assurance: 2, maxAge: 900_000 }],
            moved: [computer.machine.id, peer.machine.id],
        });
    },
);

test(
    "publish a renamed machine's new relay name while it serves, without a restart",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // serve a computer routing its names through the residency's relay
        const world = await TestComputer.world();
        const relay = await TestComputer.relay(world);
        const { computer } = await openComputer(world, world.owner.account, "work", {
            relay: relay.url,
        });

        // rename the machine as itself and find it published under the new name
        const { machine, device } = computer;
        const accounts = createClient(
            accountPackage,
            {
                url: PackageMount.url(world.universe.origin, accountPackage.id),
                fetch: machine.fetch(accountPackage.id),
            },
            device.environment,
        );
        await accounts.machine.rename({
            scope: world.owner.account,
            id: machine.id,
            requestId: RequestId.create(computer.environment),
            name: "renamed",
        });
        const universe = world.universe.storage.database;
        await computer.waitUntil(universe, async () => {
            const [published] = await universe
                .select({ url: accountObject.endpoint.table.url })
                .from(accountObject.endpoint.table)
                .where(eq(accountObject.endpoint.table.parentId, machine.id));

            return published?.url === `https://renamed--${world.owner.handle}.${DOMAINS.machine}`;
        });
        expect(computer.reports).toEqual([]);
    },
);

test(
    "close a space before its buckets while one of them is open, failing nothing busy",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // store a file in a bucket of a space
        const world = await TestComputer.world();
        const served = await openComputer(world, world.owner.account, "work");
        const files = await keepBucket(served, { name: "files", spec: {}, package: echo });
        const host = await buckets(served);
        await (await host.open({ bucketId: files })).put("notes/a.txt", "first");

        // close the computer with the buckets
        await served.computer.close();
        expect(served.computer.reports).toEqual([]);
    },
);

test(
    "refuse a package the machine neither ships nor links in its process, its installation serving nothing",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // publish the board, which no machine ships, and install it in process on a computer
        const { home, organizations } = await TestComputer.home({ organizations: [PUBLISHER] });
        const { world } = home;
        const served = await openComputer(world, found(organizations, PUBLISHER.handle), "board");
        const { computer, login, space } = served;
        const build = await TestDistribution.fixture("board");
        await home.publish(build);
        const refused = await computer
            .install(login.id, space, build, "board", { isolation: "host" })
            .catch((error: unknown) => (error instanceof Error ? error.message : String(error)));

        // fail the rollout for good, reporting why once, and serve nothing of the board in the machine's process
        const message =
            "@destack/machine-board-fixture runs on the host, which only releases the machine ships or links may";
        expect({
            refused,
            captured: world.telemetry.take().map((captured) => captured.message),
        }).toEqual({
            refused: `board failed to apply: @destack/stack PROCESS_ISOLATION_UNTRUSTED: ${message}`,
            captured: [message],
        });
    },
);

test(
    "serve a space whose installation fails to start",
    { timeout: SCENARIO_MILLISECONDS },
    async () => {
        // place the region the regional fixture requires in a space, then install the fixture there
        const { home } = await TestComputer.home();
        const { world } = home;
        const { computer, login, space } = await openComputer(
            world,
            world.owner.account,
            "regional",
        );
        const build = await TestDistribution.fixture("regional");
        await home.publish(build);
        const released = PackageManifest.package(build.manifest);
        const { device } = computer;
        const settings = createClient(
            settingPackage,
            (await device.launcher(login.id)).endpoint(
                settingPackage.id,
                await device.locate(space),
            ),
            device.environment,
        );
        const region = await settings.setting.create({
            scope: space,
            packageId: released.id,
            name: "region",
            mode: "set",
            value: "eu",
            release: released.version,
            requestId: RequestId.create(computer.environment),
        });
        await computer.install(login.id, space, build, "regional");

        // remove the region, which the installation's start then cannot read
        await settings.setting.delete({
            ...settingObject.setting.target(region),
            requestId: RequestId.create(computer.environment),
        });

        // restart the computer, which starts the space, then wait for its reconcile to leave the installation unserved
        const restarted = await computer.restart();
        const started = await world.simulation.wait(restarted.machine.space(space));
        const lagging = "installation regional does not serve its revision yet";
        const settled = await world.simulation.wait(
            (async () => {
                for await (const state of started.health.watch()) {
                    if (state.reason === lagging) {
                        return state;
                    }
                }

                throw new Error("the space's health stopped before its reconcile");
            })(),
        );

        // ask the installation, which answers not served yet with when to retry
        const answered = await world.simulation.wait(
            restarted.machine.serve(
                new Request(
                    `${InstallationMount.url("http://127.0.0.1", space, "regional")}${PackageMount.path(released.id)}/`,
                ),
            ),
        );

        // the space starts, reports the installation that failed first, and refuses it with a retry
        expect({
            status: settled.status,
            reason: settled.reason,
            reported: Failure.of(await restarted.reported).message,
            answered: [answered.status, answered.headers.get("retry-after")],
        }).toEqual({
            status: "starting",
            reason: lagging,
            reported: "installation regional serves nothing: its server failed to start",
            answered: [503, "1"],
        });
    },
);
