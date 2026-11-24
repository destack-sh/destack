import {
    Installation,
    installation,
    PROCESS_RUNTIME,
    build,
    installationRevision,
} from "@destack/stack/object";
import { Plan } from "@destack/resource";
import { ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { testBuild, testIdentity } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { eq } from "@destack/db";
import { argument, plural, t } from "@destack/locale";
import { Identifier, present, Digest, nth, schema } from "@destack/schema";
import { InstallationId, principal, accessRelationship, Relationship } from "@destack/access";
import { account, residency, space } from "@destack/account/object";
import { UNIVERSE_NODE } from "@destack/universe/test";
import {
    defineObject,
    definePackage,
    field,
    type InstanceOf,
    PACKAGE_KIND,
    PACKAGE_EXPORT,
    Package,
} from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import { implementPackage } from "@destack/object/server";
import { MemoryBuild } from "@destack/package/test";
import { PackageVersion } from "@destack/package";
import { defineBucket } from "@destack/bucket";
import { describeBucket } from "@destack/bucket/inspect";
import { testHost } from "@destack/test/host";
import { PackageManifest } from "@destack/package/manifest";
import * as object from "@destack/stack/object";
import {
    fixtureBuild,
    ids,
    insertRelease,
    insertInstallation,
    readInstalled,
    TestSpace,
} from "../test/index.ts";
import stackPackage from "@destack/stack/package";

/** How long a failed pass gets to retry: 30 s, inside the minute before its failures report. */
const RETRY_MILLISECONDS = 30_000;

/** The board package, linked into the machine, whose cards other linked packages refer to. */
const BOARD = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000e3",
    name: "@example/board",
    version: "2026.9.0",
});

/** The pins package, linked into the machine, whose pins refer to the board's cards. */
const PINS = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000e4",
    name: "@example/pins",
    version: "2026.9.0",
});

/** A card on the board. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: space,
        fields: { title: field.string(schema.string().min(1)) },
        access: { permissions: ["read"] },
    },
    (method) => ({ get: method.get("read") }),
    { package: BOARD },
);

/** A pin on one of the board's cards. */
const pin = defineObject(
    {
        name: "pin",
        plural: "pins",
        title: t`${plural(argument("count"), { one: "Pin", other: "Pins" })}`,
        scope: space,
        fields: { card: field.reference(card) },
        access: { permissions: ["read"] },
    },
    (method) => ({ get: method.get("read") }),
    { package: PINS },
);

/** The board's handle, serving its cards. */
const boardPackage = definePackage({ ...testIdentity, objects: [card] }, { package: BOARD });

/** The pins' handle, granted nothing of the board. */
const pinsPackage = definePackage({ ...testIdentity, objects: [pin] }, { package: PINS });

/** The pins' handle granted to read the space's cards, waiting for a space serving the board. */
const grantedPinsPackage = definePackage(
    {
        ...testIdentity,
        objects: [pin],
        access: { permissions: { space: { space: [card.permission("read")] } } },
    },
    { package: PINS },
);

/** Open a space whose machine links the board and a pins handle, serving neither yet. */
async function openLinked(pins: typeof pinsPackage | typeof grantedPinsPackage) {
    // build both packages and run their servers in process
    const builds = {
        board: await testBuild(boardPackage, testHost().crypto),
        pins: await testBuild(pins, testHost().crypto),
    };
    const fixture = await TestSpace.open({
        servers: [
            implementPackage(boardPackage, { objects: [card] }),
            implementPackage(pins, { objects: [pin] }),
        ],
        linked: [builds.board, builds.pins],
    });
    await fixture.serve();

    return { fixture, builds };
}

/** Read an installation's conditions and the revisions its rollout targets, applied and serves. */
async function readRolled(fixture: TestSpace, id: Identifier<"installation">) {
    // read the installation with its rollout
    const rolled = await readInstalled(fixture.database, id);
    const conditions = Object.fromEntries(
        Object.entries(rolled.conditions).map(([name, { status, reason, message }]) => [
            name,
            [status, reason, message],
        ]),
    );

    return {
        conditions,
        target: present(rolled.targetRevisionId, "the targeted revision"),
        applied: rolled.appliedRevisionId,
        served: rolled.servedRevisionId,
    };
}

test("install an application into a space as its owner, and refuse a stranger whether its name is taken or free", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const version = ObjectReference.columns(
        "version",
        await fixture.releaseRow(ids.package, "2026.9.0"),
    );
    const create = (user: string | Subject, name = "notes") =>
        fixture
            .as(user)
            .client(stackPackage)
            .installation.create({
                scope: ids.space,
                requestId: RequestId.create(fixture.machine.environment),
                packageId: ids.package,
                ...version,
                name,
            });

    // install the package as an unmanaged application under its name, rolled out from the start
    const created = await create(fixture.owner.id);
    const [installed] = await fixture.database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, created.id));
    const [rolled] = await fixture.database
        .select({ id: installation.table.id })
        .from(installation.table)
        .where(eq(installation.table.id, created.id));
    expect([
        installed?.managerInstallationId,
        installed?.name,
        installed?.packageId,
        installed?.status,
        rolled?.id,
    ]).toEqual([null, "notes", ids.package, "enabled", created.id]);

    // refuse a caller with no role in the space alike for a free and a taken name
    for (const name of ["tasks", "notes"]) {
        expect(
            await refusal(create(principal.user.reference(Scope.universe.id, "stranger"), name)),
        ).toEqual(["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${ids.space}' not found`]);
    }
});

test("find and list only the enabled installations not being deleted, as a machine opens their views", async () => {
    // add an enabled, a suspended and a deleting installation that follow a revision each, a moment after the stack
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.simulation.jump(fixture.simulation.clock.now() + 1);
    const now = fixture.machine.clock.now();
    const keep = async (
        name: string,
        state: { readonly status?: "suspended"; readonly deletedAt?: number },
    ) => {
        const id = Identifier.create("installation", fixture.environment);
        const revisionId = Identifier.create("installation-revision", fixture.environment);
        await insertInstallation(
            database,
            {
                id,
                scope: ids.space,
                packageId: Identifier.create("package", fixture.environment),
                name,
                version: undefined,
                installer: fixture.owner.subject,
            },
            now,
        );
        const release = {
            package: ids.package,
            version: "2026.9.0",
            image: Digest.parse("e".repeat(64)),
        };
        const pulled = await insertRelease(
            database,
            ids.space,
            release,
            [],
            fixture.machine.environment,
        );
        await database.transaction(
            async (transaction) =>
                await transaction.insert(installationRevision.table).values({
                    id: revisionId,
                    scope: ids.space,
                    installationId: id,
                    buildId: pulled.id,
                    dependencies: [],
                    states: {},
                    release: release.version,
                    declarations: Digest.parse("e".repeat(64)),
                    generation: 1,
                    bindings: [],
                    digest: Digest.parse("e".repeat(64)),
                    createdAt: now,
                    updatedAt: now,
                }),
        );
        const { status, deletedAt } = state;
        await database.transaction(async (transaction) => {
            await transaction
                .update(installation.table)
                .set({ targetRevisionId: revisionId })
                .where(eq(installation.table.id, id));
            const changed = {
                ...(status === undefined ? {} : { status }),
                ...(deletedAt === undefined ? {} : { deletedAt }),
            };
            if (Object.keys(changed).length > 0) {
                await transaction
                    .update(installation.table)
                    .set(changed)
                    .where(eq(installation.table.id, id));
            }
        });
    };
    await keep("notes", {});
    await keep("tasks", { status: "suspended" });
    await keep("board", { deletedAt: now });

    // open only the enabled one by its name, and list it with the space's stack
    const found = async (name: string) =>
        (await Installation.find(database, ids.space, name, "launchable"))?.name;
    const listed = await Installation.list(database, ids.space, "launchable");
    expect([
        await found("notes"),
        await found("tasks"),
        await found("board"),
        listed.map((entry) => entry.name),
    ]).toEqual(["notes", undefined, undefined, ["stack", "notes"]]);
});

test("roll an installation back by following an earlier revision's build, each time", async () => {
    // install notes as the owner following each release in turn
    const fixture = await TestSpace.open();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    let installed: InstanceOf<typeof object.installation> | undefined;
    const install = async (version: string) => {
        // publish the release and run it
        await fixture.publish(await fixtureBuild(ids.package, version, fixture.environment.crypto));
        const row = ObjectReference.columns(
            "version",
            await fixture.releaseRow(ids.package, version),
        );
        const requestId = RequestId.create(fixture.machine.environment);
        installed =
            installed === undefined
                ? await installer.installation.create({
                      scope: ids.space,
                      requestId,
                      packageId: ids.package,
                      name: "notes",
                      ...row,
                  })
                : await installer.installation.update({
                      scope: ids.space,
                      id: installed.id,
                      requestId,
                      revision: (
                          await installer.installation.get({ scope: ids.space, id: installed.id })
                      ).revision,
                      ...row,
                  });
        await fixture.simulation.idle();
        const { id } = installed;

        // approve the plan its release waits on
        const waiting = await readInstalled(fixture.database, id);
        if (waiting.plan !== null) {
            await owner.installation.approve({
                scope: ids.space,
                id,
                requestId: RequestId.create(fixture.machine.environment),
                plan: await Plan.digest(waiting.plan, fixture.environment.crypto),
            });
            await fixture.simulation.idle();
        }

        return { id, revisionId: (await readInstalled(fixture.database, id)).targetRevisionId };
    };
    const buildOf = async (id: Identifier<"installation-revision"> | null) =>
        (
            await fixture.database
                .select({ buildId: installationRevision.table.buildId })
                .from(installationRevision.table)
                .where(eq(installationRevision.table.id, present(id, "the revision")))
        )[0]?.buildId;

    // follow an earlier revision's build, as a roll back does
    const rollBack = async (
        id: Identifier<"installation">,
        revisionId: Identifier<"installation-revision"> | null,
    ) => {
        const built = present(await buildOf(revisionId), "the revision's build");
        const current = await installer.installation.get({ scope: ids.space, id });
        await installer.installation.update({
            scope: ids.space,
            id,
            requestId: RequestId.create(fixture.machine.environment),
            revision: current.revision,
            ...ObjectReference.columns("version", build.reference(ids.space, built)),
            directory: null,
        });
        await fixture.simulation.idle();

        return await buildOf((await readInstalled(fixture.database, id)).targetRevisionId);
    };

    // apply three releases, then roll back to the second
    const first = await install("2026.9.0");
    const second = await install("2026.9.1");
    await install("2026.9.2");
    const back = await rollBack(first.id, second.revisionId);
    const [repinned] = await fixture.database
        .select({ versionType: installation.table.versionType })
        .from(installation.table)
        .where(eq(installation.table.id, first.id));
    const { versionType } = present(repinned, "the installation");

    // follow the first release again, then roll back to the second again
    await install("2026.9.0");
    const reused = await rollBack(first.id, second.revisionId);

    // target a revision of the second release's build both times, following that build
    const built = await buildOf(second.revisionId);
    expect({ back, versionType, reused }).toEqual({
        back: built,
        versionType: "build",
        reused: built,
    });
});

test("name a residency home's declared installation by its derived identifier", async () => {
    // install notes in the eu residency's home through its stack, then serve it again
    const home = await TestSpace.residency();
    await home.serve();
    const reader = await fixtureBuild(ids.package, "2026.9.0", home.environment.crypto);
    const installed = await home.install(reader);
    await home.simulation.idle();

    // read the residency's installation relationships at the universe
    const related = await Relationship.read(
        home.universe.storage.database.snapshot(),
        residency.reference(Scope.universe.id, "eu"),
        { relation: "installation" },
    );
    const { name } = PackageManifest.package(reader.manifest);
    const declared = await InstallationId.of(
        { id: home.id },
        name.slice(name.lastIndexOf("/") + 1),
        home.environment.crypto,
    );
    expect({ installed, related: related.map((row) => row.subjectId) }).toEqual({
        installed: declared,
        related: [],
    });
});

test("relate a residency home's installation at the universe once a partition heals", async () => {
    // declare a package whose installations ask for what a residency's installation has
    const released = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e2",
        name: "@example/registry",
        version: "2026.9.0",
    });
    const registry = Package.of(released, {
        objects: [],
        access: {
            permissions: {
                space: {
                    universe: [
                        account.permission("entitle"),
                        account.permission("serve"),
                        residency.permission("locate"),
                    ],
                },
            },
        },
    });
    const reader = await MemoryBuild.of(
        PackageVersion.of(registry),
        [
            {
                ...PACKAGE_KIND.identity,
                name: PACKAGE_EXPORT,
                description: describePackage({ ...registry, ...testIdentity }),
            },
        ],
        testHost().crypto,
    );

    // install it in the eu residency's home while its machine cannot call the universe
    const home = await TestSpace.residency();
    await home.serve();
    await home.publish(reader);
    await home.simulation.idle();
    await home.machine.resolve(released.id, released.version);
    const { network } = home.simulation;
    network.partition(home.machine.id, UNIVERSE_NODE);
    const installed = await home.install(reader);
    const snapshot = home.universe.storage.database.snapshot();
    const eu = present(
        await snapshot.read(residency.table).findFirst({ where: { code: "eu" } }),
        "the eu residency",
    );
    const related = async () =>
        (
            await Relationship.read(snapshot, residency.reference(Scope.universe.id, eu.id), {
                relation: "installation",
            })
        ).map((row) => row.subjectId);
    const partitioned = await related();

    // heal the partition, and let the failed pass retry and relate it
    network.heal(home.machine.id, UNIVERSE_NODE);
    await home.simulation.run(home.environment.clock.now() + RETRY_MILLISECONDS);
    expect({ partitioned, healed: await related() }).toEqual({
        partitioned: [],
        healed: [installed],
    });
});

test("relate an in-process installation to its bound bucket until uninstalled", async () => {
    // ship a server binding a bucket, and install it in the space
    const released = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e1",
        name: "@example/boxes",
        version: "2026.9.0",
    });
    const box = defineBucket({ name: "box", spec: {} }, { package: released });
    const boxes = Package.of(released, { objects: [], resources: [box] });
    const reader = await MemoryBuild.of(
        PackageVersion.of(boxes),
        [
            {
                ...PACKAGE_KIND.identity,
                name: PACKAGE_EXPORT,
                description: describePackage({ ...boxes, ...testIdentity }),
            },
            {
                ...box.kind.identity,
                provisioned: true,
                name: box.name,
                description: describeBucket(box),
            },
        ],
        testHost().crypto,
    );
    const fixture = await TestSpace.open({
        servers: [implementPackage(boxes, { objects: [] })],
        linked: [reader],
    });
    await fixture.serve();
    const installed = await fixture.install(reader);

    // read the installation's consumer relationships, by the bucket its binding targets
    const consumed = async () => {
        const rows = await fixture.database
            .select({ object: accessRelationship.objectId, subject: accessRelationship.subjectId })
            .from(accessRelationship)
            .where(eq(accessRelationship.relation, "consumer"));

        return rows.filter((row) => row.subject === installed);
    };
    const [deployed] = await fixture.database
        .select()
        .from(object.deployment.table)
        .where(eq(object.deployment.table.installationId, installed));
    const live = present(deployed, "the installation's deployment");
    const [revision] = await fixture.database
        .select()
        .from(object.installationRevision.table)
        .where(eq(object.installationRevision.table.id, live.revisionId));
    const bound = {
        target: nth(present(revision, "its revision").bindings, 0).target,
        runtime: live.runtime,
    };
    const serving = await consumed();

    // uninstall it, which stops its process deployment
    await fixture.apply([], { installations: {} });
    expect({ bound, serving, uninstalled: await consumed() }).toEqual({
        bound: { target: present(bound, "the box binding").target, runtime: PROCESS_RUNTIME },
        serving: [{ object: present(bound, "the box binding").target, subject: installed }],
        uninstalled: [],
    });
});

test("deploy a linked package referring to an unserved linked package's type", async () => {
    // install the pins alone, while the space serves no board
    const { fixture, builds } = await openLinked(pinsPackage);
    const installed = await fixture.install(builds.pins);

    // apply and serve the pins' revision, reading the board's card from its linked release
    const { target, ...rolled } = await readRolled(fixture, installed);
    expect(rolled).toEqual({
        conditions: {
            Bound: ["true", "Bound", "every bound target is ready"],
            Ready: ["true", "Deployed", "1 workloads deployed"],
            Synced: ["true", "ReconcileSuccess", ""],
            Serving: ["true", "Served", `serving revision ${target}`],
        },
        applied: target,
        served: target,
    });
});

test("deploy an installation waiting for a granted type once the space serves it", async () => {
    // install the pins granted the board's cards, which waits for a space serving the board
    const { fixture, builds } = await openLinked(grantedPinsPackage);
    const installed = await fixture.install(builds.pins);
    const waiting = await readRolled(fixture, installed);

    // install the board beside them as an application, touching nothing of the pins
    await fixture
        .as(fixture.owner.subject)
        .client(stackPackage)
        .installation.create({
            scope: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            packageId: BOARD.id,
            ...ObjectReference.columns("version", undefined),
            name: "board",
        });
    await fixture.simulation.idle();

    // wake the waiting pins once the space serves the board, and deploy them
    const deployed = await readRolled(fixture, installed);
    const { target } = waiting;
    expect({ waiting, deployed }).toEqual({
        waiting: {
            conditions: {
                Bound: ["true", "Bound", "every bound target is ready"],
                Ready: [
                    "false",
                    "WaitingForSpace",
                    `waiting for the space to serve ${BOARD.id}/card`,
                ],
                Synced: ["true", "ReconcileSuccess", ""],
                Serving: ["true", "Served", `serving revision ${target}`],
            },
            target,
            applied: null,
            served: target,
        },
        deployed: {
            conditions: {
                Bound: ["true", "Bound", "every bound target is ready"],
                Ready: ["true", "Deployed", "1 workloads deployed"],
                Synced: ["true", "ReconcileSuccess", ""],
                Serving: ["true", "Served", `serving revision ${target}`],
            },
            target,
            applied: target,
            served: target,
        },
    });
});
