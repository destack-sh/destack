import { Plan } from "@destack/resource";
import { expect, refusal, test } from "@destack/test";
import { schema, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { CallError } from "@destack/object";
import { eq, Snapshot } from "@destack/db";
import type { BuildReference, InstallationSelection } from "../src/declare/index.ts";
import { serveSpace } from "../src/server/index.ts";
import { build, Installation, installation, installationRevision } from "../src/object/index.ts";
import { PackageId } from "@destack/package";
import {
    ARCHIVE,
    fixtureBuild,
    ids,
    reconcile,
    StackFixture,
    TestSpaceDatabaseHost,
} from "../src/test/index.ts";
import { PackageStore } from "@destack/build/store";

/** The checkout the stack follows. */
const selection = {
    kind: "checkout" as const,
    machine: ids.machine,
    checkout: schema.identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    directory: "." as const,
};

/** The key the installation controller applies the space's stack under. */
const STACK_KEY = { scope: ids.space, role: "stack" };

/** Select a release of a package by its version. */
function release(version: string) {
    return { kind: "release" as const, version };
}

test("apply a submitted stack build and open its installations' views", async () => {
    const fixture = await StackFixture.open();
    const owner = (await fixture.serve({ isControlled: false }))(ids.owner);
    const store = serveSpace(
        await fixture.options({ providers: [await TestSpaceDatabaseHost.provider()] }),
    ).store;

    // create the space's stack installation following a checkout
    const stack = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.stackPackage,
        role: "stack",
        selection,
        export: "personal",
        parameters: {},
        name: "stack",
    });
    const submission = {
        spaceId: ids.space,
        id: stack.id,
        selection,
        build: await fixture.recordBuild(await fixtureBuild(ids.stackPackage, "2026.9.0")),
        evaluation: {
            export: "personal",
            parameters: {},
            definition: {
                roles: { reviewer: { description: "Review notes", permissions: [] } },
                installations: {
                    notes: {
                        package: { id: ids.package, name: "@example/notes", version: "2026.9.0" },
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
    const definition = await fixture.pin(submission.evaluation.definition);
    const evaluation = { ...submission.evaluation, definition };
    await owner.installation.submit({
        ...submission,
        evaluation,
        requestId: RequestId.create(Date.now()),
    });
    await reconcile(store, "installation", STACK_KEY);
    const applied = await owner.installation.get({ spaceId: ids.space, id: stack.id });
    expect([applied.appliedRevisionId === null, applied.conditions["ready"]?.reason]).toEqual([
        false,
        "Applied",
    ]);
    expect(await owner.installation.plan({ spaceId: ids.space, id: stack.id })).toEqual({
        steps: [],
    });

    // apply the notes installation, then open a view its applied revision declares and refuse one it does not
    const listed = (await owner.installation.list({ spaceId: ids.space })).items.find(
        (entry) => entry.name === "notes",
    );
    const notes = present(listed, "the notes installation");
    await reconcile(store, "installation", { id: notes.id });
    await fixture.serveInstalled(store, ids.space);
    await reconcile(store, "installation", { id: notes.id });
    expect(
        await owner.installation.open({
            spaceId: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: notes.revisionId,
        manifest: await fixture.registry.find(ids.package, "2026.9.0"),
        view: "home",
        description: {
            entrypoint: "view.js",
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            packages: [],
        },
        path: "/",
        permissions: {},
        home: ids.home,
    });
    await expect(
        owner.installation.open({ spaceId: ids.space, id: notes.id, view: "settings", path: "/" }),
    ).rejects.toEqual(
        new CallError("NOT_FOUND", {
            defined: true,
            message: "notes declares no view settings",
        }),
    );
});

test("apply an earlier build submitted again after a later one, approving each plan that waits", async () => {
    const fixture = await StackFixture.open();
    const owner = (await fixture.serve({ isControlled: false }))(ids.owner);
    const store = serveSpace(
        await fixture.options({ providers: [await TestSpaceDatabaseHost.provider()] }),
    ).store;
    const stack = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "stack",
        selection,
        export: "personal",
        parameters: {},
        name: "stack",
    });

    // submit a build declaring one role and one declaring both
    const submit = async (role: string, version: string) => {
        const submission = {
            spaceId: ids.space,
            id: stack.id,
            selection,
            build: await fixture.recordBuild(await fixtureBuild(ids.package, version)),
            evaluation: {
                export: "personal",
                parameters: {},
                definition: { roles: { [role]: { description: role, permissions: [] } } },
            },
        };
        await owner.installation.submit({ ...submission, requestId: RequestId.create(Date.now()) });

        // apply the revision, approving the plan it waits on first
        await reconcile(store, "installation", STACK_KEY);
        const read = () => owner.installation.get({ spaceId: ids.space, id: stack.id });
        const isWaiting = (await read()).plan !== null;
        if (isWaiting) {
            const waiting = await owner.installation.plan({ spaceId: ids.space, id: stack.id });
            await owner.installation.approve({
                spaceId: ids.space,
                id: stack.id,
                requestId: RequestId.create(Date.now()),
                plan: await Plan.digest(waiting),
            });
            await reconcile(store, "installation", STACK_KEY);
        }

        return { revision: (await read()).revisionId, isWaiting };
    };
    const first = await submit("reviewer", "2026.9.0");
    const second = await submit("guest", "2026.10.0");
    const third = await submit("reviewer", "2026.9.0");

    // apply the first build's revision once more, each submission after the first waiting on its plan
    const applied = await owner.installation.get({ spaceId: ids.space, id: stack.id });
    expect([
        [first.isWaiting, second.isWaiting, third.isWaiting],
        applied.revisionId,
        applied.appliedRevisionId,
        applied.conditions["ready"]?.reason,
    ]).toEqual([[false, true, true], first.revision, first.revision, "Applied"]);
});

test("install an application from a checkout by submitting its build, following its revision's views and declarations", async () => {
    const fixture = await StackFixture.open();
    const owner = (await fixture.serve({ isControlled: false }))(ids.owner);
    const store = serveSpace(
        await fixture.options({ providers: [await TestSpaceDatabaseHost.provider()] }),
    ).store;
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "application",
        selection,
        name: "notes",
    });

    // submit the checkout's build, apply it, and open the view it mounts
    const reader = await fixtureBuild(ids.package, "2026.9.0");
    const recorded = await fixture.recordBuild(reader);
    await owner.installation.submit({
        spaceId: ids.space,
        id: notes.id,
        requestId: RequestId.create(Date.now()),
        selection,
        build: recorded,
    });
    await reconcile(store, "installation", { id: notes.id });
    await fixture.serveInstalled(store, ids.space);
    await reconcile(store, "installation", { id: notes.id });
    const followed = await owner.installation.get({ spaceId: ids.space, id: notes.id });
    expect(
        await owner.installation.open({
            spaceId: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: followed.revisionId,
        manifest: await PackageStore.digest(reader.manifest),
        view: "home",
        description: {
            entrypoint: "view.js",
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            packages: [],
        },
        path: "/",
        permissions: {},
        home: ids.home,
    });
    const declared = await owner.declaration.list({ buildId: recorded.build });
    expect(declared.items.map(({ kind, name, description }) => [kind, name, description])).toEqual([
        ["command", "archive", ARCHIVE],
    ]);
});

test("refuse submitting a build that has not succeeded or that the space does not have", async () => {
    // record a pending build of the checkout
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const owner = (await fixture.serve())(ids.owner);
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "application",
        selection,
        name: "notes",
    });
    const now = Date.now();
    const pending = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000b1");
    const missing = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000b2");
    await database.insert(build.table).values({
        id: pending,
        scope: ids.space,
        source: { ...selection },
        outputs: ["bun"],
        createdAt: now,
        updatedAt: now,
    });

    // refuse the pending build and a build of no row
    const submit = (id: typeof pending) =>
        owner.installation
            .submit({
                spaceId: ids.space,
                id: notes.id,
                requestId: RequestId.create(Date.now()),
                selection,
                build: { kind: "build", build: id },
            })
            .catch((error: unknown) => error);
    expect([await submit(pending), await submit(missing)]).toEqual([
        new CallError("CONFLICT", { defined: true, message: `build ${pending} is pending` }),
        new CallError("NOT_FOUND", { defined: true, message: `no build ${missing}` }),
    ]);
});

test("wait for an approval of an application's upgrade across every release since the one it applied, refusing another digest", async () => {
    const fixture = await StackFixture.open();
    const owner = (await fixture.serve({ isControlled: false }))(ids.owner);
    const store = serveSpace(
        await fixture.options({ providers: [await TestSpaceDatabaseHost.provider()] }),
    ).store;
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "application",
        selection: release("2026.9.0"),
        name: "notes",
    });
    const submit = (version: string) =>
        owner.installation.submit({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(Date.now()),
            selection: release(version),
            build: release(version),
        });
    const read = () => owner.installation.get({ spaceId: ids.space, id: notes.id });

    // apply the first release and wait on the upgrade to the last
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await reconcile(store, "installation", { id: notes.id });
        await fixture.serveInstalled(store, ids.space);
        await reconcile(store, "installation", { id: notes.id });
    };
    await submit("2026.9.0");
    await deploy();
    const deployed = (await read()).conditions["ready"]?.reason;
    await submit("2026.11.0");
    await deploy();
    expect([deployed, (await read()).conditions["ready"]?.reason]).toEqual([
        "Deployed",
        "AwaitingApproval",
    ]);
    const waiting = await owner.installation.plan({ spaceId: ids.space, id: notes.id });
    expect(waiting).toEqual({
        steps: [
            {
                action: "create",
                target: "installation/notes/release/2026.10.0/api/procedure/count",
                risk: "safe",
                detail: "add procedure count",
            },
            {
                action: "delete",
                target: "installation/notes/release/2026.11.0/object/note/relation/editor",
                risk: "backward-incompatible",
                detail: "remove: data stored under it no longer applies",
            },
        ],
    });

    // refuse another digest and deploy the approved release
    const digest = await Plan.digest(waiting);
    const approve = (plan: string) =>
        owner.installation.approve({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(Date.now()),
            plan,
        });
    const other = "0".repeat(64);
    const refused = await refusal(approve(other));
    const approved = await approve(digest);
    await deploy();
    const applied = await read();
    expect([
        refused,
        approved,
        applied.appliedRevisionId === applied.revisionId,
        applied.plan,
    ]).toEqual([
        ["CONFLICT", `plan ${digest} waits for approval, not ${other}`],
        waiting,
        true,
        null,
    ]);
});

test("refuse a build of another package, and block an upgrade from a release that is not earlier", async () => {
    // keep a checkout build of a fork's package the installation does not install
    const fixture = await StackFixture.open();
    const fork = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f0");
    const owner = (await fixture.serve({ isControlled: false }))(ids.owner);
    const store = serveSpace(
        await fixture.options({ providers: [await TestSpaceDatabaseHost.provider()] }),
    ).store;
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "application",
        selection: release("2026.9.0"),
        name: "notes",
    });
    const submit = (submitted: InstallationSelection, reference: BuildReference) =>
        refusal(
            owner.installation.submit({
                spaceId: ids.space,
                id: notes.id,
                requestId: RequestId.create(Date.now()),
                selection: submitted,
                build: reference,
            }),
        );
    const read = () => owner.installation.get({ spaceId: ids.space, id: notes.id });

    // refuse the fork's build at once
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await reconcile(store, "installation", { id: notes.id });
        await fixture.serveInstalled(store, ids.space);
        await reconcile(store, "installation", { id: notes.id });
    };
    await submit(release("2026.9.0"), release("2026.9.0"));
    await deploy();
    const deployed = (await read()).conditions["ready"]?.reason;
    const refused = await submit(
        selection,
        await fixture.recordBuild(await fixtureBuild(fork, "2026.9.0")),
    );

    // block the release that upgrades from itself until another submission
    const blocked = await submit(release("2026.12.0"), release("2026.12.0"));
    await deploy();
    const ready = (await read()).conditions["ready"];
    expect([deployed, refused, blocked, ready?.reason, ready?.message]).toEqual([
        "Deployed",
        ["BAD_REQUEST", `notes installs ${ids.package}, submitted a build of ${fork}`],
        "done",
        "NoUpgrade",
        "notes 2026.12.0 upgrades from 2026.12.0, which is not earlier",
    ]);
});

test("follow a distribution tag, recording the version of the release it points at in the revision", async () => {
    // install the package following its latest tag
    const fixture = await StackFixture.open();
    const owner = (await fixture.serve())(ids.owner);
    const latest = { kind: "tag" as const, tag: "latest" };
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "application",
        selection: latest,
        name: "notes",
    });

    // submit the tag
    await owner.installation.submit({
        spaceId: ids.space,
        id: notes.id,
        requestId: RequestId.create(Date.now()),
        selection: latest,
        build: latest,
    });
    const followed = await owner.installation.get({ spaceId: ids.space, id: notes.id });
    const [revision] = await fixture.database
        .select({ source: build.table.source, manifest: build.table.manifest })
        .from(installationRevision.table)
        .innerJoin(build.table, eq(build.table.id, installationRevision.table.buildId))
        .where(eq(installationRevision.table.id, present(followed.revisionId, "the revision")));
    const manifest = await fixture.registry.find(ids.package, "latest");
    expect([followed.selection, revision]).toEqual([
        latest,
        {
            source: { kind: "release", package: ids.package, version: "2026.10.0", manifest },
            manifest,
        },
    ]);
});

test("open the build each package's installation in a space follows, refusing a package the space does not install", async () => {
    // install two packages at different releases
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const now = Date.now();
    const follow = async (packageId: PackageId, version: string, suffix: string) => {
        const id = schema
            .identifier("installation")
            .parse(`installation-01996ab0-0000-7000-8000-0000000000${suffix}`);
        const revisionId = schema
            .identifier("installation-revision")
            .parse(`installation-revision-01996ab0-0000-7000-8000-0000000000${suffix}`);
        await database.insert(installation.table).values({
            id,
            scope: ids.space,
            packageId,
            role: "application",
            selection: { kind: "release", version },
            name: `notes-${suffix}`,
            createdAt: now,
            updatedAt: now,
        });
        await database.insert(installationRevision.table).values({
            id: revisionId,
            scope: ids.space,
            installationId: id,
            buildId: (await fixture.release(packageId, version)).id,
            dependencies: [],
            digest: suffix.repeat(32),
            createdAt: now,
            updatedAt: now,
        });
        await database
            .update(installation.table)
            .set({ revisionId })
            .where(eq(installation.table.id, id));
    };
    await follow(ids.package, "2026.9.0", "e1");
    await follow(ids.stackPackage, "2026.10.0", "e2");

    // open each package's build and refuse a package the space does not install
    const builds = await fixture.builds();
    const version = (packageId: PackageId) =>
        Installation.reader(
            Snapshot.live(database),
            ids.space,
            packageId,
            undefined,
            builds.packages,
        ).then(
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

test("serve an installation a call names at its mount by its identifier or its name, refusing a name no installation has and a suspended installation", async () => {
    // install notes in the space
    const fixture = await StackFixture.open();
    const { database } = fixture;
    const owner = (await fixture.serve())(ids.owner);
    const notes = await owner.installation.create({
        spaceId: ids.space,
        requestId: RequestId.create(Date.now()),
        packageId: ids.package,
        role: "application",
        selection,
        name: "notes",
    });

    // read it by its identifier and its name, and refuse another name and a path naming no space
    const served = async (spaceId: string, named: string) =>
        refusal(
            Installation.requireServed(database, { spaceId, installation: named }).then(
                ({ id }) => id,
            ),
        );
    const byId = await Installation.requireServed(database, {
        spaceId: ids.space,
        installation: notes.id,
    });
    const byName = await Installation.requireServed(database, {
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
        ["NOT_FOUND", `this machine serves no installation tasks in ${ids.space}`],
        ["NOT_FOUND", "this machine serves no installation notes in nowhere"],
    ]);

    // suspend it and answer its callers to wait
    await owner.installation.update({
        spaceId: ids.space,
        id: notes.id,
        requestId: RequestId.create(Date.now()),
        revision: notes.revision,
        status: "suspended",
    });
    expect(await served(ids.space, "notes")).toEqual([
        "UNAVAILABLE",
        `installation notes in ${ids.space} is not served yet`,
    ]);
});
