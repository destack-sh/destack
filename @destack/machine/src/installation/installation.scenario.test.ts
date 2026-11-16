import { Plan } from "@destack/resource";
import type { Installation } from "@destack/object";
import { ObjectReference } from "@destack/sync";
import { TestRegistry } from "../test/build.ts";
import { testIdentity } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { eq, Snapshot } from "@destack/db";
import { Identifier, present, Digest, nth } from "@destack/schema";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { accessRelationship, InstallationId, principal, Relationship } from "@destack/access";
import { account, residency } from "@destack/account/object";
import { UNIVERSE_NODE } from "@destack/control/test";
import { installation, PACKAGE_KIND, PACKAGE_EXPORT, Package } from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import { implementPackage } from "@destack/object/server";
import { MemoryBuild } from "@destack/package/test";
import { PackageVersion } from "@destack/package";
import { defineBucket } from "@destack/bucket";
import { describeBucket } from "@destack/bucket/inspect";
import { testHost } from "@destack/test/host";
import { PROCESS_RUNTIME } from "@destack/stack/object";
import { PackageManifest } from "@destack/package/manifest";
import * as object from "@destack/stack/object";
import { build, installationRevision, Rollout, rollout } from "@destack/stack/object";
import {
    fixtureBuild,
    ids,
    insertRelease,
    insertRollout,
    readInstalled,
    TestSpace,
} from "../test/index.ts";
import stackPackage from "@destack/stack/package";

/** How long a failed pass gets to retry: 30 s, inside the minute before its failures report. */
const RETRY_MILLISECONDS = 30_000;

test("install an application into a space as its owner, and refuse a stranger whether its name is taken or free", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const create = (user: string | Subject, name = "notes") =>
        fixture
            .as(user)
            .client(objectPackage)
            .installation.create({
                scope: ids.space,
                requestId: RequestId.create(fixture.machine.environment),
                packageId: ids.package,
                ...ObjectReference.columns("follows", TestRegistry.row("release", "2026.9.0")),
                name,
            });

    // install the package as an unmanaged application under its name, rolled out from the start
    const created = await create(fixture.owner.id);
    const [installed] = await fixture.database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, created.id));
    const [rolled] = await fixture.database
        .select({ id: rollout.table.installationId })
        .from(rollout.table)
        .where(eq(rollout.table.installationId, created.id));
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
        await insertRollout(
            database,
            {
                id,
                scope: ids.space,
                packageId: Identifier.create("package", fixture.environment),
                name,
                follows: TestRegistry.row("release", "2026.9.0"),
                installer: fixture.owner.subject,
            },
            now,
        );
        const release = {
            package: ids.package,
            version: "2026.9.0",
            lock: Digest.parse("e".repeat(64)),
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
                .update(rollout.table)
                .set({ targetRevisionId: revisionId })
                .where(eq(rollout.table.installationId, id));
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
        (await Rollout.find(database, ids.space, name))?.installation.name;
    const listed = await Rollout.list(database, ids.space);
    expect([
        await found("notes"),
        await found("tasks"),
        await found("board"),
        listed.map((entry) => entry.rollout.installation.name),
    ]).toEqual(["notes", undefined, undefined, ["stack", "notes"]]);
});

test("roll an installation back by following an earlier revision's build, each time", async () => {
    // install notes as the owner following each release in turn
    const fixture = await TestSpace.open();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    let installed: Installation | undefined;
    const install = async (version: string) => {
        // release the version and follow it
        await fixture.registry.release(
            await fixtureBuild(ids.package, version, fixture.environment.crypto),
        );
        const follows = ObjectReference.columns("follows", TestRegistry.row("release", version));
        const requestId = RequestId.create(fixture.machine.environment);
        installed =
            installed === undefined
                ? await installer.installation.create({
                      scope: ids.space,
                      requestId,
                      packageId: ids.package,
                      name: "notes",
                      ...follows,
                  })
                : await installer.installation.update({
                      scope: ids.space,
                      id: installed.id,
                      requestId,
                      revision: (
                          await installer.installation.get({ scope: ids.space, id: installed.id })
                      ).revision,
                      ...follows,
                  });
        await fixture.simulation.idle();
        const { id } = installed;

        // approve the plan its release waits on
        const { installation: waiting } = await readInstalled(fixture.database, id);
        if (waiting.plan !== null) {
            await owner.rollout.approve({
                spaceId: ids.space,
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
            ...ObjectReference.columns("follows", build.reference(ids.space, built)),
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
        .select({ followsType: installation.table.followsType })
        .from(installation.table)
        .where(eq(installation.table.id, first.id));
    const { followsType } = present(repinned, "the installation");

    // follow the first release again, then roll back to the second again
    await install("2026.9.0");
    const reused = await rollBack(first.id, second.revisionId);

    // target a revision of the second release's build both times, following that build
    const built = await buildOf(second.revisionId);
    expect({ back, followsType, reused }).toEqual({
        back: built,
        followsType: "build",
        reused: built,
    });
});

test("identify a residency home's declared installation by its name, and forward its unrelating to the residency on each pass", async () => {
    // install notes in the eu residency's home through its stack, then serve it again
    const home = await TestSpace.residency();
    await home.serve();
    const reader = await fixtureBuild(ids.package, "2026.9.0", home.environment.crypto);
    const installed = await home.install(reader);
    await home.simulation.idle();

    // read the residency's installation relationships at the universe
    const related = await Relationship.read(
        Snapshot.live(home.universe.storage.database),
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

test("relate a residency home's declared installation at the universe once a pass after a partition arrives there", async () => {
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
    const { network } = home.simulation;
    network.partition(home.machine.id, UNIVERSE_NODE);
    const installed = await home.install(reader);
    const snapshot = Snapshot.live(home.universe.storage.database);
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

test("relate a shipped server as consumer of the bucket it binds while its revision serves in process, and unrelate it once uninstalled", async () => {
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
