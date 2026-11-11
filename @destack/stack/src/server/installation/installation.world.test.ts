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
import { PROCESS_RUNTIME } from "../../object/index.ts";
import { PackageManifest } from "@destack/package/manifest";
import * as object from "../../object/index.ts";
import { installationRevision, Rollout, rollout } from "../../object/index.ts";
import {
    fixtureBuild,
    ids,
    insertRelease,
    insertRollout,
    readInstalled,
    TestSpace,
} from "../../test/index.ts";
import stackPackage from "../../package.ts";

/** How long a failed pass gets to retry: 30 s, inside the minute before its failures report. */
const RETRY_MILLISECONDS = 30_000;

test("install an application into a space as its owner, and refuse a stranger whether its name is taken or free", async () => {
    const fixture = await TestSpace.open();
    const served = await fixture.serve();
    const create = (user: string | Subject, name = "notes") =>
        served
            .as(user)
            .client(objectPackage)
            .installation.create({
                scope: ids.space,
                requestId: RequestId.create(fixture.machine.environment),
                packageId: ids.package,
                selection: { kind: "release", version: "2026.9.0" },
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
                selection: { kind: "release", version: "2026.9.0" },
                installer: fixture.owner.subject,
            },
            now,
        );
        const release = {
            package: ids.package,
            version: "2026.9.0",
            manifest: Digest.parse("e".repeat(64)),
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
                .update(installation.table)
                .set({ revisionId })
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
        (await Rollout.find(database, ids.space, name))?.installation.name;
    const listed = await Rollout.list(database, ids.space);
    expect([
        await found("notes"),
        await found("tasks"),
        await found("board"),
        listed.map((entry) => entry.rollout.installation.name),
    ]).toEqual(["notes", undefined, undefined, ["stack", "notes"]]);
});

test("roll an installation back to the revision applied before, a reused one too, and refuse one without", async () => {
    // add an application with three revisions, applied by hand while no controller runs
    const fixture = await TestSpace.open();
    const served = await fixture.serve({ isControlled: false });
    const owner = served.as(fixture.owner.subject).client(stackPackage);
    const installer = served.as(fixture.owner.subject).client(objectPackage);
    const installed = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection: { kind: "release", version: "2026.9.2" },
        name: "notes",
    });
    const revise = async (version: string) => {
        const id = Identifier.create("installation-revision", fixture.environment);
        const now = fixture.machine.clock.now();
        const built = await fixture.release(ids.package, version);
        await fixture.database.transaction(
            async (transaction) =>
                await transaction.insert(installationRevision.table).values({
                    id,
                    scope: ids.space,
                    installationId: installed.id,
                    buildId: built.id,
                    dependencies: [],
                    states: {},
                    generation: 1,
                    bindings: [],
                    digest: Digest.parse(version.replaceAll(".", "").padEnd(64, "0")),
                    createdAt: now,
                    updatedAt: now,
                }),
        );

        return id;
    };
    const first = await revise("2026.9.0");
    const second = await revise("2026.9.1");
    const third = await revise("2026.9.2");
    const read = async () => {
        const [row] = await fixture.database
            .select()
            .from(rollout.table)
            .where(eq(rollout.table.installationId, installed.id));

        return present(row, "the rollout");
    };
    const apply = async (revisionId: Identifier<"installation-revision">) => {
        const fields = Rollout.applied(await read(), revisionId);
        await fixture.database.transaction(async (transaction) => {
            await transaction
                .update(installation.table)
                .set({ revisionId })
                .where(eq(installation.table.id, installed.id));
            await transaction
                .update(rollout.table)
                .set(fields)
                .where(eq(rollout.table.installationId, installed.id));
        });
    };
    const rollBack = () =>
        owner.rollout.rollBack({
            spaceId: ids.space,
            id: installed.id,
            requestId: RequestId.create(fixture.machine.environment),
        });

    // refuse a rollback before a second revision applied
    await apply(first);
    const refused = await refusal(rollBack());

    // follow the revision applied before, with its release pinned
    await apply(second);
    await apply(third);
    const back = await rollBack();
    const { revisionId } = await readInstalled(fixture.database, installed.id);
    const [reselected] = await fixture.database
        .select({ selection: installation.table.selection })
        .from(installation.table)
        .where(eq(installation.table.id, installed.id));
    const { selection } = present(reselected, "the installation");

    // follow a newer revision again after the older one is applied again
    await apply(second);
    await apply(first);
    const reused = await rollBack();

    expect({ refused, back, followed: { revisionId, selection }, reused }).toEqual({
        refused: [
            "NOT_FOUND",
            "REVISION_NOT_FOUND",
            "previous revision of installation 'notes' not found",
        ],
        back: { revisionId: second },
        followed: { revisionId: second, selection: { kind: "release", version: "2026.9.1" } },
        reused: { revisionId: second },
    });
});

test("identify a residency home's declared installation by its name, and forward its unrelating to the residency on each pass", async () => {
    // install notes in the eu residency's home through its stack, then serve it again
    const home = await TestSpace.residency();
    const served = await home.serve();
    const build = await fixtureBuild(ids.package, "2026.9.0", home.environment.crypto);
    const installed = await home.install(served, build);
    await home.simulation.idle();

    // read the residency's installation relationships at the universe
    const related = await Relationship.readOn(
        Snapshot.live(home.universe.storage.database),
        residency.reference(Scope.universe.id, "eu"),
        { relation: "installation" },
    );
    const { name } = PackageManifest.package(build.manifest);
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
    const build = await MemoryBuild.of(
        PackageVersion.of(registry),
        [
            {
                ...PACKAGE_KIND.identity,
                name: PACKAGE_EXPORT,
                description: describePackage(registry),
            },
        ],
        testHost().crypto,
    );

    // install it in the eu residency's home while its machine cannot call the universe
    const home = await TestSpace.residency();
    const served = await home.serve();
    const { network } = home.simulation;
    network.partition(home.machine.id, UNIVERSE_NODE);
    const installed = await home.install(served, build);
    const snapshot = Snapshot.live(home.universe.storage.database);
    const eu = present(
        await snapshot.read(residency.table).findFirst({ where: { code: "eu" } }),
        "the eu residency",
    );
    const related = async () =>
        (
            await Relationship.readOn(snapshot, residency.reference(Scope.universe.id, eu.id), {
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
    const build = await MemoryBuild.of(
        PackageVersion.of(boxes),
        [
            {
                ...PACKAGE_KIND.identity,
                name: PACKAGE_EXPORT,
                description: describePackage(boxes),
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
        linked: [build],
    });
    const served = await fixture.serve();
    const installed = await fixture.install(served, build);

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
    await fixture.apply(served, [], { installations: {} });
    expect({ bound, serving, uninstalled: await consumed() }).toEqual({
        bound: { target: present(bound, "the box binding").target, runtime: PROCESS_RUNTIME },
        serving: [{ object: present(bound, "the box binding").target, subject: installed }],
        uninstalled: [],
    });
});
