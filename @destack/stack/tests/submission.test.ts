import { type BuildReference } from "@destack/stack/declare";
import { type InstallationSelection } from "@destack/object";
import objectPackage from "@destack/object/package";
import { Plan } from "@destack/resource";
import { expect, refusal, test } from "@destack/test";
import { schema, present, Digest } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { CallError } from "@destack/object";
import { eq, Snapshot } from "@destack/db";
import { serveSpace } from "../src/server/index.ts";
import { build, installationRevision, Rollout, rollout } from "../src/object/index.ts";
import { Package, PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { defineSetting } from "@destack/setting/declare";
import { describeSetting } from "@destack/setting/inspect";
import settingPackage from "@destack/setting/package";
import {
    readInstalled,
    readStack,
    ARCHIVE,
    fixtureBuild,
    ids,
    insertRollout,
    reconcile,
    TestSpace,
} from "../src/test/index.ts";
import { PackageStore } from "@destack/build/store";
import stackPackage from "../src/package.ts";

/** The checkout the stack follows. */
const selection = {
    kind: "checkout" as const,
    machine: ids.machine,
    checkout: schema.identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    directory: "." as const,
};

/** Select a release of a package by its version. */
function release(version: string) {
    return { kind: "release" as const, version };
}

test("apply a submitted stack build and open its installations' views", async () => {
    const fixture = await TestSpace.open();
    const serving = await fixture.serve({ isControlled: false });
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const store = serveSpace(await fixture.options()).store;

    // follow a checkout with the stack installation the space's birth gave it
    const stack = await readStack(fixture.database);
    const submission = {
        spaceId: ids.space,
        id: stack.id,
        selection,
        build: await fixture.recordBuild(
            await fixtureBuild(
                stack.installation.packageId,
                "2026.9.0",
                fixture.environment.crypto,
            ),
        ),
        evaluation: {
            export: "personal",
            parameters: {},
            definition: {
                roles: { reviewer: { description: "Review notes", permissions: [] } },
                installations: {
                    notes: {
                        packageId: ids.package,
                        selection: { kind: "release", version: "2026.9.0" },
                        status: "enabled",
                        name: "notes",
                        bindings: {},
                        compute: {},
                        capabilities: [],
                        directories: [],
                        tags: {},
                    },
                },
            },
        },
    };

    // submit the build pinned as its evaluation is and have the stack controller apply its safe plan without approval
    const definition = await fixture.resolve(submission.evaluation.definition);
    const evaluation = { ...submission.evaluation, definition };
    await owner.rollout.submit({
        ...submission,
        evaluation,
        requestId: RequestId.create(fixture.machine.environment),
    });
    await reconcile(store, "rollout", { id: (await readStack(store.database)).id });
    const applied = await readInstalled(fixture.database, stack.id);
    expect([
        applied.appliedRevisionId === null,
        applied.installation.conditions["Ready"]?.reason,
    ]).toEqual([false, "Applied"]);
    expect(await owner.rollout.plan({ spaceId: ids.space, id: stack.id })).toEqual({
        steps: [],
    });

    // apply the notes installation, then open a view its applied revision declares and refuse one it does not
    const listed = await Rollout.find(fixture.database, ids.space, "notes");
    const notes = present(listed, "the notes installation");
    await reconcile(store, "rollout", { id: notes.id });
    await fixture.serveInstalled(store, ids.space);
    await reconcile(store, "rollout", { id: notes.id });
    const revised = await readInstalled(fixture.database, notes.id);
    expect(
        await owner.rollout.open({
            spaceId: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: revised.revisionId,
        manifest: await fixture.registry.find(ids.package, "2026.9.0"),
        view: "home",
        description: {
            entrypoint: "view.js",
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            categories: [],
            packages: [],
        },
        path: "/",
        permissions: {},
        home: fixture.owner.home,
    });
    expect(
        await refusal(
            owner.rollout.open({
                spaceId: ids.space,
                id: notes.id,
                view: "settings",
                path: "/",
            }),
        ),
    ).toEqual(["NOT_FOUND", "VIEW_NOT_FOUND", "notes declares no view settings"]);
});

test("apply an earlier build submitted again after a later one, approving each plan that waits", async () => {
    const fixture = await TestSpace.open();
    const serving = await fixture.serve({ isControlled: false });
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const store = serveSpace(await fixture.options()).store;
    const stack = await readStack(fixture.database);

    // submit a build declaring one role and one declaring both
    const submit = async (role: string, version: string) => {
        const submission = {
            spaceId: ids.space,
            id: stack.id,
            selection,
            build: await fixture.recordBuild(
                await fixtureBuild(
                    stack.installation.packageId,
                    version,
                    fixture.environment.crypto,
                ),
            ),
            evaluation: {
                export: "personal",
                parameters: {},
                definition: { roles: { [role]: { description: role, permissions: [] } } },
            },
        };
        await owner.rollout.submit({
            ...submission,
            requestId: RequestId.create(fixture.machine.environment),
        });

        // apply the revision, approving the plan it waits on first
        await reconcile(store, "rollout", { id: (await readStack(store.database)).id });
        const read = () => readInstalled(fixture.database, stack.id);
        const isWaiting = (await read()).installation.plan !== null;
        if (isWaiting) {
            const waiting = await owner.rollout.plan({ spaceId: ids.space, id: stack.id });
            await owner.rollout.approve({
                spaceId: ids.space,
                id: stack.id,
                requestId: RequestId.create(fixture.machine.environment),
                plan: await Plan.digest(waiting, fixture.environment.crypto),
            });
            await reconcile(store, "rollout", { id: (await readStack(store.database)).id });
        }

        return { revision: (await read()).revisionId, isWaiting };
    };
    const first = await submit("reviewer", "2026.9.0");
    const second = await submit("guest", "2026.10.0");
    const third = await submit("reviewer", "2026.9.0");

    // apply the first build's revision once more, each submission after the first waiting on its plan
    const applied = await readInstalled(fixture.database, stack.id);
    expect([
        [first.isWaiting, second.isWaiting, third.isWaiting],
        applied.revisionId,
        applied.appliedRevisionId,
        applied.installation.conditions["Ready"]?.reason,
    ]).toEqual([[false, true, true], first.revision, first.revision, "Applied"]);
});

test("install an application from a checkout by submitting its build, following its revision's views and declarations", async () => {
    const fixture = await TestSpace.open();
    const serving = await fixture.serve({ isControlled: false });
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const installer = serving.as(fixture.owner.subject).client(objectPackage);
    const store = serveSpace(await fixture.options()).store;
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection,
        name: "notes",
    });

    // submit the checkout's build, apply it, and open the view it mounts
    const reader = await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto);
    const recorded = await fixture.recordBuild(reader);
    await owner.rollout.submit({
        spaceId: ids.space,
        id: notes.id,
        requestId: RequestId.create(fixture.machine.environment),
        selection,
        build: recorded,
    });
    await reconcile(store, "rollout", { id: notes.id });
    await fixture.serveInstalled(store, ids.space);
    await reconcile(store, "rollout", { id: notes.id });
    const followed = await readInstalled(fixture.database, notes.id);
    expect(
        await owner.rollout.open({
            spaceId: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: followed.revisionId,
        manifest: await PackageStore.digest(reader.manifest, fixture.environment.crypto),
        view: "home",
        description: {
            entrypoint: "view.js",
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            categories: [],
            packages: [],
        },
        path: "/",
        permissions: {},
        home: fixture.owner.home,
    });
    const declared = await owner.declaration.list({ buildId: recorded.build });
    expect(declared.items.map(({ kind, name, description }) => [kind, name, description])).toEqual([
        ["command", "archive", ARCHIVE],
        ["relation", "editor", {}],
    ]);
});

test("refuse submitting a build that has not succeeded or that the space does not have", async () => {
    // record a pending build of the checkout
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const serving = await fixture.serve();
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const installer = serving.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection,
        name: "notes",
    });
    const now = fixture.machine.clock.now();
    const pending = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000b1");
    const missing = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000b2");
    await database.transaction(
        async (transaction) =>
            await transaction.insert(build.table).values({
                id: pending,
                scope: ids.space,
                source: { ...selection },
                outputs: ["bun"],
                createdAt: now,
                updatedAt: now,
            }),
    );

    // refuse the pending build and a build of no row
    const submit = (id: typeof pending) =>
        owner.rollout
            .submit({
                spaceId: ids.space,
                id: notes.id,
                requestId: RequestId.create(fixture.machine.environment),
                selection,
                build: { kind: "build", build: id },
            })
            .catch((error: unknown) => error);
    expect([await submit(pending), await submit(missing)]).toEqual([
        new CallError("CONFLICT", {
            reason: "BUILD_NOT_READY",
            domain: "@destack/stack",
            defined: true,
            message: `build ${pending} is pending`,
        }),
        new CallError("NOT_FOUND", {
            reason: "BUILD_NOT_FOUND",
            domain: "@destack/stack",
            defined: true,
            message: `build ${missing} not found`,
        }),
    ]);
});

test("wait for an approval of an application's upgrade planned from the release it applied, refusing another digest", async () => {
    const fixture = await TestSpace.open();
    const serving = await fixture.serve({ isControlled: false });
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const installer = serving.as(fixture.owner.subject).client(objectPackage);
    const store = serveSpace(await fixture.options()).store;
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection: release("2026.9.0"),
        name: "notes",
    });
    const submit = (version: string) =>
        owner.rollout.submit({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(fixture.machine.environment),
            selection: release(version),
            build: release(version),
        });
    const read = () => readInstalled(fixture.database, notes.id);

    // apply the first release and wait on the upgrade to the last
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await reconcile(store, "rollout", { id: notes.id });
        await fixture.serveInstalled(store, ids.space);
        await reconcile(store, "rollout", { id: notes.id });
    };
    await submit("2026.9.0");
    await deploy();
    const deployed = (await read()).installation.conditions["Ready"]?.reason;
    await submit("2026.11.0");
    await deploy();
    expect([deployed, (await read()).installation.conditions["Ready"]?.reason]).toEqual([
        "Deployed",
        "AwaitingApproval",
    ]);
    const waiting = await owner.rollout.plan({ spaceId: ids.space, id: notes.id });
    expect(waiting).toEqual({
        steps: [
            {
                action: "create",
                target: "installation/notes/release/2026.11.0/procedure/count",
                risk: "safe",
                detail: "add procedure",
            },
            {
                action: "delete",
                target: "installation/notes/release/2026.11.0/relation/editor",
                risk: "backward-incompatible",
                detail: "remove relation: earlier releases still serve it",
            },
        ],
    });

    // refuse another digest and deploy the approved release
    const digest = await Plan.digest(waiting, fixture.environment.crypto);
    const approve = (plan: string) =>
        owner.rollout.approve({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(fixture.machine.environment),
            plan,
        });
    const other = Digest.parse("0".repeat(64));
    const refused = await refusal(approve(other));
    const approved = await approve(digest);
    await deploy();
    const applied = await read();
    expect([
        refused,
        approved,
        applied.appliedRevisionId === applied.revisionId,
        applied.installation.plan,
    ]).toEqual([
        ["CONFLICT", "PLAN_MISMATCH", `plan ${digest} is waiting for approval, not ${other}`],
        waiting,
        true,
        null,
    ]);
});

test("refuse a build of another package, and deploy a release that is not later without a plan", async () => {
    // record a checkout build of a fork's package the installation does not install
    const fixture = await TestSpace.open();
    const fork = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f0");
    const serving = await fixture.serve({ isControlled: false });
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const installer = serving.as(fixture.owner.subject).client(objectPackage);
    const store = serveSpace(await fixture.options()).store;
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection: release("2026.9.0"),
        name: "notes",
    });
    const submit = (submitted: InstallationSelection, reference: BuildReference) =>
        refusal(
            owner.rollout.submit({
                spaceId: ids.space,
                id: notes.id,
                requestId: RequestId.create(fixture.machine.environment),
                selection: submitted,
                build: reference,
            }),
        );
    const read = () => readInstalled(fixture.database, notes.id);

    // refuse the fork's build at once
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await reconcile(store, "rollout", { id: notes.id });
        await fixture.serveInstalled(store, ids.space);
        await reconcile(store, "rollout", { id: notes.id });
    };
    await submit(release("2026.9.0"), release("2026.9.0"));
    await deploy();
    const deployed = (await read()).installation.conditions["Ready"]?.reason;
    const refused = await submit(
        selection,
        await fixture.recordBuild(await fixtureBuild(fork, "2026.9.0", fixture.environment.crypto)),
    );

    // deploy a later release declaring the same, then roll back to the first, each without a plan
    await submit(release("2026.10.0"), release("2026.10.0"));
    await deploy();
    const upgraded = await read();
    await submit(release("2026.9.0"), release("2026.9.0"));
    await deploy();
    const rolledBack = await read();
    expect([
        deployed,
        refused,
        [upgraded.installation.conditions["Ready"]?.reason, upgraded.installation.plan],
        [rolledBack.installation.conditions["Ready"]?.reason, rolledBack.installation.plan],
    ]).toEqual([
        "Deployed",
        [
            "BAD_REQUEST",
            "PACKAGE_MISMATCH",
            `notes installs ${ids.package}, but the submitted build is ${fork}`,
        ],
        ["Deployed", null],
        ["Deployed", null],
    ]);
});

test("follow a distribution tag, recording the version of the release it points at in the revision", async () => {
    // install the package following its latest tag
    const fixture = await TestSpace.open();
    const serving = await fixture.serve();
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const installer = serving.as(fixture.owner.subject).client(objectPackage);
    const latest = { kind: "tag" as const, tag: "latest" };
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection: latest,
        name: "notes",
    });

    // submit the tag, the space resolving it in its world
    await fixture.simulation.drive(
        owner.rollout.submit({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(fixture.machine.environment),
            selection: latest,
            build: latest,
        }),
    );
    const followed = await readInstalled(fixture.database, notes.id);
    const selected = await installer.installation.get({ scope: ids.space, id: notes.id });
    const [revision] = await fixture.database
        .select({ source: build.table.source, manifest: build.table.manifest })
        .from(installationRevision.table)
        .innerJoin(build.table, eq(build.table.id, installationRevision.table.buildId))
        .where(eq(installationRevision.table.id, present(followed.revisionId, "the revision")));
    const manifest = await fixture.registry.find(ids.package, "latest");
    expect([selected.selection, revision]).toEqual([
        latest,
        {
            source: { kind: "release", package: ids.package, version: "2026.10.0", manifest },
            manifest,
        },
    ]);
});

test("open the build each package's installation in a space follows, refusing a package the space does not install", async () => {
    // install two packages at different releases
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const now = fixture.machine.clock.now();
    const follow = async (packageId: PackageId, version: string, suffix: string) => {
        const id = schema
            .identifier("installation")
            .parse(`installation-01996ab0-0000-7000-8000-0000000000${suffix}`);
        const revisionId = schema
            .identifier("installation-revision")
            .parse(`installation-revision-01996ab0-0000-7000-8000-0000000000${suffix}`);
        await insertRollout(
            database,
            {
                id,
                scope: ids.space,
                packageId,
                name: `notes-${suffix}`,
                selection: { kind: "release", version },
                installer: fixture.owner.subject,
            },
            now,
        );
        const built = await fixture.release(packageId, version);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(installationRevision.table).values({
                    id: revisionId,
                    scope: ids.space,
                    installationId: id,
                    buildId: built.id,
                    dependencies: [],
                    states: {},
                    digest: Digest.parse(suffix.repeat(32)),
                    createdAt: now,
                    updatedAt: now,
                }),
        );
        await database.transaction(
            async (transaction) =>
                await transaction
                    .update(rollout.table)
                    .set({ revisionId })
                    .where(eq(rollout.table.id, id)),
        );
    };
    await follow(ids.package, "2026.9.0", "e1");
    await follow(ids.stackPackage, "2026.10.0", "e2");

    // open each package's build and refuse a package the space does not install
    const builds = await fixture.builds();
    const version = (packageId: PackageId) =>
        Rollout.reader(Snapshot.live(database), ids.space, packageId, undefined, builds).then(
            (reader) => reader.manifest.package.version,
            (error: unknown) => (error instanceof Error ? error.message : error),
        );
    const absent = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f2");
    expect([
        await version(ids.package),
        await version(ids.stackPackage),
        await version(absent),
    ]).toEqual([
        "2026.9.0",
        "2026.10.0",
        `no installation in ${ids.space} follows a build of ${absent}`,
    ]);
});

test("serve the installation of a call at its mount by its identifier or its name, refusing a name no installation has and a suspended installation", async () => {
    // install notes in the space
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const serving = await fixture.serve();
    const installer = serving.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection,
        name: "notes",
    });

    // read it by its identifier and its name, and refuse another name and a path naming no space
    const served = async (spaceId: string, named: string) =>
        refusal(
            Rollout.requireServed(database, { spaceId, installation: named }).then(({ id }) => id),
        );
    const byId = await Rollout.requireServed(database, {
        spaceId: ids.space,
        installation: notes.id,
    });
    const byName = await Rollout.requireServed(database, {
        spaceId: ids.space,
        installation: "notes",
    });
    expect([
        byId.id,
        byName.id,
        await served(ids.space, "tasks"),
        await served("nowhere", "notes"),
    ]).toEqual([
        notes.id,
        notes.id,
        [
            "NOT_FOUND",
            "INSTALLATION_NOT_HERE",
            `this machine does not serve installation tasks in ${ids.space}`,
        ],
        [
            "NOT_FOUND",
            "INSTALLATION_NOT_HERE",
            "this machine does not serve installation notes in nowhere",
        ],
    ]);

    // suspend it and answer its callers to wait
    await installer.installation.update({
        scope: ids.space,
        id: notes.id,
        requestId: RequestId.create(fixture.machine.environment),
        revision: notes.revision,
        status: "suspended",
    });
    expect(await served(ids.space, "notes")).toEqual([
        "SERVICE_UNAVAILABLE",
        "INSTALLATION_NOT_SERVED",
        `installation notes in ${ids.space} is not served yet`,
    ]);
});

/** The region a space must place for the regional package, which declares no default. */
const region = defineSetting({
    name: "region",
    title: "Region",
    description: "The region the space serves.",
    schema: schema.string().min(1),
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});

test("refuse submitting a build whose space settings without a default the space leaves unset, before applying it", async () => {
    // record a checkout build of a package declaring a required region
    const fixture = await TestSpace.open();
    const serving = await fixture.serve({ isControlled: false });
    const owner = serving.as(fixture.owner.subject).client(stackPackage);
    const installer = serving.as(fixture.owner.subject).client(objectPackage);
    const regional = Package.parse({ id: ids.package, name: "@test/package", version: "2026.9.0" });
    const declaring = await MemoryBuild.of(
        regional,
        [
            {
                kind: "setting",
                kindPackageId: settingPackage.id,
                name: region.name,
                description: { ...describeSetting(region), package: regional },
            },
        ],
        fixture.environment.crypto,
    );
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        selection,
        name: "notes",
    });

    // refuse its submission while the space places no region
    const submitted = await refusal(
        owner.rollout.submit({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(fixture.machine.environment),
            selection,
            build: await fixture.recordBuild(declaring),
        }),
    );
    expect([submitted, (await readInstalled(fixture.database, notes.id)).revisionId]).toEqual([
        [
            "PRECONDITION_FAILED",
            "SETTING_UNSET",
            `setting ${ids.package}/region has no default and is not set`,
        ],
        null,
    ]);
});
