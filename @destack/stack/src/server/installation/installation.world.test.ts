import { expect, refusal, test } from "@destack/test";
import { eq, Snapshot } from "@destack/db";
import { Identifier, present, Digest } from "@destack/schema";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { accessRelationship, InstallationId, principal, Relationship } from "@destack/access";
import { account, entitlement, residency } from "@destack/account/object";
import { UNIVERSE_NODE } from "@destack/universe/test";
import {
    installation,
    type ObjectType,
    PACKAGE_DECLARATION,
    PACKAGE_EXPORT,
    PackageHandle,
} from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import { implementPackage } from "@destack/object/server";
import { MemoryBuild } from "@destack/package/test";
import { Package } from "@destack/package";
import { defineBucket } from "@destack/bucket";
import { describeBucket } from "@destack/bucket/inspect";
import { RESOURCE_DECLARATION, RESOURCE_KIND_PACKAGE_ID } from "@destack/resource";
import { testHost } from "@destack/test/host";
import { PROCESS_RUNTIME } from "../../object/index.ts";
import { PackageManifest } from "@destack/package/manifest";
import * as object from "../../object/index.ts";
import { installationRevision, Rollout, rollout } from "../../object/index.ts";
import { fixtureBuild, ids, insertRelease, insertRollout, TestSpace } from "../../test/index.ts";
import stackPackage from "../../package.ts";

/** How long a failed pass gets to retry: 30 s, inside the minute before its failures report. */
const RETRY_MILLISECONDS = 30_000;

test("install an application into a space as its owner, and refuse a stranger whether its name is taken or free", async () => {
    const fixture = await TestSpace.open();
    const served = await fixture.serve();
    const create = (user: string | Subject, name = "notes") =>
        served
            .as(user)
            .client(stackPackage)
            .rollout.create({
                spaceId: ids.space,
                requestId: RequestId.create(fixture.machine.environment),
                packageId: ids.package,
                kind: "package",
                selection: { kind: "release", version: "2026.9.0" },
                name,
            });

    // install the package as an application under its name, the installation installed by the owner
    const created = await create(fixture.owner.id);
    const [installed] = await fixture.database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, created.id));
    expect([
        created.kind,
        installed?.origin,
        installed?.name,
        installed?.packageId,
        installed?.status,
    ]).toEqual(["package", "installed", "notes", ids.package, "enabled"]);

    // refuse a caller with no role in the space alike for a free and a taken name
    for (const name of ["tasks", "notes"]) {
        expect(
            await refusal(create(principal.user.reference(Scope.universe.id, "stranger"), name)),
        ).toEqual(["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${ids.space} not found`]);
    }
});

test("find and list only the enabled installations not being deleted, as a machine opens their views", async () => {
    // add an enabled, a suspended and a deleting installation that follow a revision each, a moment after the stack
    const fixture = await TestSpace.open();
    const { database } = fixture;
    fixture.simulation.clock.advance(1);
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
                    digest: Digest.parse("e".repeat(64)),
                    createdAt: now,
                    updatedAt: now,
                }),
        );
        const { status, deletedAt } = state;
        await database.transaction(async (transaction) => {
            await transaction
                .update(rollout.table)
                .set({ revisionId, ...(deletedAt === undefined ? {} : { deletedAt }) })
                .where(eq(rollout.table.id, id));
            if (status !== undefined) {
                await transaction
                    .update(installation.table)
                    .set({ status })
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
    const installed = await owner.rollout.create({
        spaceId: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        kind: "package",
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
            .where(eq(rollout.table.id, installed.id));

        return present(row, "the rollout");
    };
    const apply = async (revisionId: Identifier<"installation-revision">) => {
        const fields = Rollout.applied(await read(), revisionId);
        await fixture.database.transaction(
            async (transaction) =>
                await transaction
                    .update(rollout.table)
                    .set({ revisionId, ...fields })
                    .where(eq(rollout.table.id, installed.id)),
        );
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
    const { revisionId, selection } = await read();

    // follow a newer revision again after the older one is applied again
    await apply(second);
    await apply(first);
    const reused = await rollBack();

    expect({ refused, back, followed: { revisionId, selection }, reused }).toEqual({
        refused: [
            "CONFLICT",
            "PREVIOUS_REVISION_MISSING",
            "installation notes has no revision before its applied one",
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
    await home.settle(served);

    // read the residency's installation relationships at the universe
    const related = await Relationship.readOn(
        Snapshot.live(home.universe.storage.database),
        residency.reference(Scope.universe.id, "eu"),
        { relation: "installation" },
    );
    const { name } = PackageManifest.package(build.manifest);
    const declared = await InstallationId.declared(
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
    const released = Package.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e2",
        name: "@example/registry",
        version: "2026.9.0",
    });
    const registry = PackageHandle.of(released, {
        objects: {},
        permissions: {
            installation: {
                universe: [
                    account.permission("serve"),
                    entitlement.permission("provide"),
                    residency.permission("locate"),
                ],
            },
        },
    });
    const build = await MemoryBuild.of(
        Package.of(registry),
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
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
    const related = async () =>
        (
            await Relationship.readOn(
                Snapshot.live(home.universe.storage.database),
                residency.reference(Scope.universe.id, "eu"),
                { relation: "installation" },
            )
        ).map((row) => row.subjectId);
    const partitioned = await related();

    // heal the partition, and let the failed pass retry and relate it
    network.heal(home.machine.id, UNIVERSE_NODE);
    await home.advance(served, home.environment.clock.now() + RETRY_MILLISECONDS);
    expect({ partitioned, healed: await related() }).toEqual({
        partitioned: [],
        healed: [installed],
    });
});

test("relate a shipped server as consumer of the bucket it binds while its revision serves in process, and unrelate it once uninstalled", async () => {
    // ship a server binding a bucket, and install it in the space
    const released = Package.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e1",
        name: "@example/boxes",
        version: "2026.9.0",
    });
    const box = defineBucket({ name: "box", spec: {} }, { package: released });
    const objects: Readonly<Record<string, ObjectType>> = {};
    const boxes = PackageHandle.of(released, { objects, resources: { box } });
    const build = await MemoryBuild.of(
        Package.of(boxes),
        [
            {
                kind: PACKAGE_DECLARATION,
                kindPackageId: objectPackage.id,
                name: PACKAGE_EXPORT,
                description: describePackage(boxes),
            },
            {
                kind: RESOURCE_DECLARATION,
                kindPackageId: RESOURCE_KIND_PACKAGE_ID,
                name: box.name,
                description: describeBucket(box),
            },
        ],
        testHost().crypto,
    );
    const fixture = await TestSpace.open({
        servers: [implementPackage(boxes, { objects: [] })],
        bundled: [build],
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
    const [bound] = await fixture.database
        .select({ target: object.binding.table.target, runtime: object.deployment.table.runtime })
        .from(object.binding.table)
        .innerJoin(
            object.deployment.table,
            eq(object.deployment.table.installationId, object.binding.table.installationId),
        )
        .where(eq(object.binding.table.installationId, installed));
    const serving = await consumed();

    // uninstall it, which stops its process deployment
    await fixture.apply(served, [], { installations: {} });
    expect({ bound, serving, uninstalled: await consumed() }).toEqual({
        bound: { target: present(bound, "the box binding").target, runtime: PROCESS_RUNTIME },
        serving: [{ object: present(bound, "the box binding").target, subject: installed }],
        uninstalled: [],
    });
});
