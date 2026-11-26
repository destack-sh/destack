import { Account } from "@destack/account/object";
import {
    installation,
    build,
    Build,
    installationRevision,
    Installation,
} from "@destack/stack/object";
import { fixtureGraph } from "@destack/build/test";
import { ObjectReference, RequestId } from "@destack/sync";
import { BuildImage } from "@destack/package/manifest";
import { Plan } from "@destack/resource";
import { expect, refusal, test } from "@destack/test";
import { Commit, schema, present, Digest, Identifier, type JsonObject } from "@destack/schema";
import { tag } from "@destack/registry/object";
import { CallError } from "@destack/object";
import { and, eq } from "@destack/db";
import { PackageVersion, PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { defineSetting } from "@destack/setting/declare";
import { describeSetting } from "@destack/setting/inspect";
import settingPackage from "@destack/setting/package";
import {
    readInstalled,
    readStack,
    ARCHIVE,
    fixtureBuild,
    fixtureSource,
    ids,
    insertInstallation,
    TestSpace,
} from "../src/test/index.ts";
import { BuildStore } from "@destack/build/store";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import { InstallationTarget } from "../src/installation/target.ts";
import { t } from "@destack/locale";

/** A space whose stack this machine has not applied yet. */
const UNAPPLIED_SPACE = "space-01996ab0-0000-7000-8000-0000000000f4";

/** A second package built from the notes checkout, implementing notes' package beside it. */
const OTHER_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f5");

/** The commit the stack's builds come from. */
const source = fixtureSource();

/** Pin an installation to a running build of its own, which deploys nothing until submitted. */
async function unbuilt(fixture: TestSpace) {
    // record a running build of a new commit, with no compile
    const id = Identifier.create("build", fixture.environment);
    const digits = id.slice(id.indexOf("-") + 1).replaceAll("-", "");
    const pending = { ...source, commit: Commit.parse(digits.padStart(40, "0")) };
    const now = fixture.machine.clock.now();
    await fixture.database.transaction(
        async (transaction) =>
            await transaction.insert(build.table).values({
                id,
                scope: ids.space,
                source: pending,
                key: Build.key({ source: pending, outputs: null, version: null }),
                commit: pending.commit,
                runnerId: fixture.machine.id,
                outputs: ["bun"],
                state: "running",
                startedAt: now,
                createdAt: now,
                updatedAt: now,
            }),
    );

    return { version: build.reference(ids.space, id) };
}

/** Follow a release of the fixture package by its version, at the forge of a space's world. */
async function release(fixture: TestSpace, version: string) {
    return { version: await fixture.releaseRow(ids.package, version) };
}

/** Write what an installation version as its columns. */
function following(target: { readonly version: ObjectReference; readonly directory?: string }) {
    return {
        ...ObjectReference.columns("version", target.version),
        directory: target.directory ?? null,
    };
}

/** Deploy an installation at a build of the space, as the space's owner. */
function deployBuild(
    fixture: TestSpace,
    id: Identifier<"installation">,
    built: { readonly build: Identifier<"build"> },
) {
    return fixture
        .as(fixture.owner.subject)
        .client(stackPackage)
        .installation.reconcile({
            scope: ids.space,
            id,
            requestId: RequestId.create(fixture.machine.environment),
            build: built.build,
        });
}

/** Follow another row with an installation, as the space's owner. */
async function refollow(
    fixture: TestSpace,
    id: Identifier<"installation">,
    target: { readonly version: ObjectReference; readonly directory?: string },
    stack: { readonly export?: string; readonly parameters?: JsonObject } = {},
) {
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const current = await installer.installation.get({ scope: ids.space, id });

    return installer.installation.update({
        scope: ids.space,
        id,
        requestId: RequestId.create(fixture.machine.environment),
        revision: current.revision,
        ...following(target),
        ...stack,
    });
}

/** Follow a build of the personal stack with the space's stack installation, as its owner. */
function followStack(
    fixture: TestSpace,
    id: Identifier<"installation">,
    built: { readonly build: Identifier<"build"> },
) {
    const version = build.reference(ids.space, built.build);

    return refollow(fixture, id, { version }, { export: "personal", parameters: {} });
}

test("apply a submitted stack build and open its installations' views", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);

    // follow a checkout with the stack installation the space's birth gave it
    const stack = await readStack(fixture.database);
    const definition = {
        roles: { reviewer: { description: "Review notes", permissions: [] } },
        installations: {
            notes: {
                packageId: ids.package,
                version: { kind: "release", version: "2026.9.0" },
                status: "enabled",
                name: "notes",
                bindings: {},
                compute: {},
                capabilities: [],
                directories: [],
                isolation: "sandbox",
                tags: {},
            },
        },
    };

    // submit a build of the stack declaring it, which the space evaluates, and have the stack controller apply its safe plan without approval
    await followStack(fixture, stack.id, await fixture.recordStack("personal", definition));
    await fixture.simulation.idle();
    const applied = await readInstalled(fixture.database, stack.id);
    expect([applied.appliedRevisionId === null, applied.conditions["Ready"]?.reason]).toEqual([
        false,
        "Applied",
    ]);
    expect(await owner.installation.plan({ scope: ids.space, id: stack.id })).toEqual({
        plan: { steps: [] },
        digest: await Plan.digest({ steps: [] }, fixture.environment.crypto),
        approval: null,
    });

    // apply the notes installation, then open a view its applied revision declares and refuse one it does not
    const listed = await Installation.find(fixture.database, ids.space, "notes", "launchable");
    const notes = present(listed, "the notes installation");
    await fixture.simulation.idle();
    const revised = await readInstalled(fixture.database, notes.id);
    expect(
        await owner.installation.open({
            scope: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: revised.targetRevisionId,
        image: await fixture.machine.resolve(ids.package, "2026.9.0"),
        view: "home",
        description: {
            entrypoint: "view.js",
            stylesheets: [],
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            opens: [],
            categories: [],
            audience: "person",
            packages: [],
            render: { mode: "client" },
        },
        path: "/",
        package: "@example/notes",
        permissions: [],
        home: fixture.owner.home,
    });
    expect(
        await refusal(
            owner.installation.open({
                scope: ids.space,
                id: notes.id,
                view: "settings",
                path: "/",
            }),
        ),
    ).toEqual(["NOT_FOUND", "VIEW_NOT_FOUND", "view 'settings' of 'notes' not found"]);
});

test("apply an earlier build submitted again after a later one, approving each plan that waits", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const stack = await readStack(fixture.database);

    // submit a build declaring one role and one declaring both
    const submit = async (role: string) => {
        const definition = { roles: { [role]: { description: role, permissions: [] } } };
        await followStack(fixture, stack.id, await fixture.recordStack("personal", definition));

        // apply the revision, approving the plan it waits on first
        await fixture.simulation.idle();
        const read = () => readInstalled(fixture.database, stack.id);
        const isWaiting = (await read()).plan !== null;
        if (isWaiting) {
            const { digest } = await owner.installation.plan({
                scope: ids.space,
                id: stack.id,
            });
            await owner.installation.approve({
                scope: ids.space,
                id: stack.id,
                requestId: RequestId.create(fixture.machine.environment),
                plan: digest,
            });
            await fixture.simulation.idle();
        }

        return { revision: (await read()).targetRevisionId, isWaiting };
    };
    const first = await submit("reviewer");
    const second = await submit("guest");
    const third = await submit("reviewer");

    // apply the first build's revision once more, each submission after the first waiting on its plan
    const applied = await readInstalled(fixture.database, stack.id);
    expect([
        [first.isWaiting, second.isWaiting, third.isWaiting],
        applied.targetRevisionId,
        applied.appliedRevisionId,
        applied.conditions["Ready"]?.reason,
    ]).toEqual([[false, true, true], first.revision, first.revision, "Applied"]);
});

test("install an application from a checkout by submitting its build, following its revision's views and declarations", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(await unbuilt(fixture)),
        name: "notes",
    });

    // submit the checkout's build, apply it, and open the view it mounts
    const reader = await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto);
    const recorded = await fixture.recordBuild(reader, await fixtureGraph(reader));
    await deployBuild(fixture, notes.id, recorded);
    await fixture.simulation.idle();
    const installed = await readInstalled(fixture.database, notes.id);
    expect(
        await owner.installation.open({
            scope: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: installed.targetRevisionId,
        image: await BuildImage.digest(
            BuildImage.of(await BuildStore.digest(reader.manifest, fixture.environment.crypto)),
            fixture.environment.crypto,
        ),
        view: "home",
        description: {
            entrypoint: "view.js",
            stylesheets: [],
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            opens: [],
            categories: [],
            audience: "person",
            packages: [],
            render: { mode: "client" },
        },
        path: "/",
        package: "@example/notes",
        permissions: [],
        home: fixture.owner.home,
    });
    const declared = await owner.declaration.list({ buildId: recorded.build });
    const sorted = declared.items.toSorted((left, right) =>
        left.moniker.localeCompare(right.moniker),
    );
    expect(sorted.map(({ kind, name, description }) => [kind, name, description])).toEqual([
        ["action", "archive", ARCHIVE],
        ["relation", "editor", {}],
    ]);
});

test("refuse submitting a build that has not succeeded or that the space does not have", async () => {
    // record a running build of the checkout
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(await unbuilt(fixture)),
        name: "notes",
    });
    const now = fixture.machine.clock.now();
    const running = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000b1");
    const missing = schema.identifier("build").parse("build-01996ab0-0000-7000-8000-0000000000b2");
    await database.transaction(
        async (transaction) =>
            await transaction.insert(build.table).values({
                id: running,
                scope: ids.space,
                source: { ...source },
                key: Build.key({ source, outputs: null, version: null }),
                commit: source.commit,
                runnerId: fixture.machine.id,
                outputs: ["bun"],
                state: "running",
                startedAt: now,
                createdAt: now,
                updatedAt: now,
            }),
    );

    // refuse the running build and a build of no row
    const submit = (id: typeof running) =>
        owner.installation
            .reconcile({
                scope: ids.space,
                id: notes.id,
                requestId: RequestId.create(fixture.machine.environment),
                build: id,
            })
            .catch((error: unknown) => error);
    expect([await submit(running), await submit(missing)]).toEqual([
        new CallError("SERVICE_UNAVAILABLE", {
            reason: "BUILD_UNAVAILABLE",
            domain: "@destack/stack",
            defined: true,
            message: `build '${running}' is unavailable: it is 'running'`,
        }),
        new CallError("NOT_FOUND", {
            reason: "BUILD_NOT_FOUND",
            domain: "@destack/stack",
            defined: true,
            message: `build '${missing}' not found`,
        }),
    ]);
});

test("wait for an approval of an application's upgrade planned from the release it applied, refusing another digest", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(await release(fixture, "2026.9.0")),
        name: "notes",
    });
    const submit = async (version: string) =>
        refollow(fixture, notes.id, await release(fixture, version));
    const read = () => readInstalled(fixture.database, notes.id);

    // apply the first release and wait on the upgrade to the last
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await fixture.simulation.idle();
    };
    await submit("2026.9.0");
    await deploy();
    const deployed = (await read()).conditions["Ready"]?.reason;
    await submit("2026.11.0");
    await deploy();
    expect([deployed, (await read()).conditions["Ready"]?.reason]).toEqual([
        "Deployed",
        "AwaitingApproval",
    ]);
    const { plan: waiting, digest } = await owner.installation.plan({
        scope: ids.space,
        id: notes.id,
    });
    expect([waiting, digest]).toEqual([
        {
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
        },
        // name the digest of exactly these steps
        await Plan.digest(waiting, fixture.environment.crypto),
    ]);

    // refuse another digest and deploy the approved release
    const approve = (plan: string) =>
        owner.installation.approve({
            scope: ids.space,
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
        approved.id,
        applied.appliedRevisionId === applied.targetRevisionId,
        applied.plan,
    ]).toEqual([
        [
            "PRECONDITION_FAILED",
            "PLAN_BLOCKED",
            `cannot approve plan '${other}': plan '${digest}' is waiting for approval`,
        ],
        notes.id,
        true,
        null,
    ]);
});

test("refuse a build of another package, and deploy a release that is not later without a plan", async () => {
    // record a checkout build of a fork's package the installation does not install
    const fixture = await TestSpace.open();
    const fork = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f0");
    await fixture.serve();
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(await release(fixture, "2026.9.0")),
        name: "notes",
    });
    const follow = async (version: string) =>
        refollow(fixture, notes.id, await release(fixture, version));
    const read = () => readInstalled(fixture.database, notes.id);

    // refuse the fork's build at once
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await fixture.simulation.idle();
    };
    await deploy();
    const deployed = (await read()).conditions["Ready"]?.reason;
    const forked = await fixtureBuild(fork, "2026.9.0", fixture.environment.crypto);
    const refused = await refusal(
        deployBuild(fixture, notes.id, await fixture.recordBuild(forked)),
    );

    // deploy a later release declaring the same, then roll back to the first, each without a plan
    await follow("2026.10.0");
    await deploy();
    const upgraded = await read();
    await follow("2026.9.0");
    await deploy();
    const rolledBack = await read();
    expect([
        deployed,
        refused,
        [upgraded.conditions["Ready"]?.reason, upgraded.plan],
        [rolledBack.conditions["Ready"]?.reason, rolledBack.plan],
    ]).toEqual([
        "Deployed",
        [
            "BAD_REQUEST",
            "PACKAGE_INVALID",
            "build is invalid: it is of '@example/notes', which 'notes' does not install",
        ],
        ["Deployed", null],
        ["Deployed", null],
    ]);
});

test("wait for a tag that has not reached the space, and for a release under the space's own tag, reporting each", async () => {
    // install the package following a tag of a home nothing serves here, and an unpublished tag of the space itself
    const homes = { elsewhere: Account.home(ids.account), own: ids.space };
    const conditions: Record<string, unknown[]> = {};
    for (const [where, home] of Object.entries(homes)) {
        const fixture = await TestSpace.open();
        await fixture.serve();
        const installer = fixture.as(fixture.owner.subject).client(stackPackage);
        const notes = await installer.installation.create({
            scope: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            packageId: ids.package,
            ...following({
                version: tag.reference(home, Identifier.create("tag", fixture.machine.environment)),
            }),
            name: "notes",
        });

        // reconcile its rollout, which finds no release to revise
        await fixture.simulation.idle();
        const waiting = await readInstalled(fixture.database, notes.id);
        const ready = waiting.conditions["Ready"];
        conditions[where] = [
            waiting.targetRevisionId,
            ready?.status,
            ready?.reason,
            ready?.message,
        ];
    }

    // wait for the other home's tag to arrive, and report that the space's own tag has no release
    expect(conditions).toEqual({
        elsewhere: [null, "false", "WaitingForTag", "waiting for its tag to reach the space"],
        own: [null, "false", "ReleaseNotPublished", "notes has no release under its tag yet"],
    });
});
test("open the build each package's installation in a space version, and none for a package the space does not install", async () => {
    // install two packages at different releases, publishing the second's
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
        await insertInstallation(
            database,
            {
                id,
                scope: ids.space,
                packageId,
                name: `notes-${suffix}`,
                version: await fixture.releaseRow(packageId, version),
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
                    ...built.recorded,
                    generation: 1,
                    bindings: [],
                    digest: Digest.parse(suffix.repeat(32)),
                    createdAt: now,
                    updatedAt: now,
                }),
        );
        await database.transaction(
            async (transaction) =>
                await transaction
                    .update(installation.table)
                    .set({ targetRevisionId: revisionId })
                    .where(eq(installation.table.id, id)),
        );
    };
    const { crypto } = fixture.machine.environment;
    await fixture.publish(
        await fixtureBuild(ids.stackPackage, "2026.10.0", crypto, "@example/stack"),
    );
    await follow(ids.package, "2026.9.0", "e1");
    await follow(ids.stackPackage, "2026.10.0", "e2");

    // open each package's build, and none for a package the space does not install
    const space = await fixture.space;
    const version = (packageId: PackageId) =>
        InstallationTarget.reader(database.snapshot(), ids.space, packageId, undefined, {
            space,
            linked: new Map(),
            shipped: new Map(),
        }).then(
            (reader) => reader?.manifest.package.version,
            (error: unknown) => (error instanceof Error ? error.message : error),
        );
    const absent = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f2");
    expect([
        await version(ids.package),
        await version(ids.stackPackage),
        await version(absent),
    ]).toEqual(["2026.9.0", "2026.10.0", undefined]);
});

test("find a served installation by identifier or name, else ask to wait or refuse", async () => {
    // install notes from its checkout's build, applied and served as the machine's start serves it
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.serve();
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(await unbuilt(fixture)),
        name: "notes",
    });
    const recorded = await fixture.recordBuild(
        await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto),
    );
    await fixture
        .as(fixture.owner.subject)
        .client(stackPackage)
        .installation.reconcile({
            scope: ids.space,
            id: notes.id,
            requestId: RequestId.create(fixture.machine.environment),
            build: recorded.build,
        });
    await fixture.simulation.idle();

    // read it by its identifier and its name, refuse another name, and answer a space before its first stack pass to wait
    const served = async (spaceId: string, named: string) =>
        refusal(
            Installation.requireServed(database, { spaceId, installation: named }).then(
                ({ id: installationId }) => installationId,
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
        await served(UNAPPLIED_SPACE, "notes"),
    ]).toEqual([
        notes.id,
        notes.id,
        [
            "NOT_FOUND",
            "INSTALLATION_NOT_FOUND",
            "installation 'tasks' served by this machine in this space not found",
        ],
        [
            "SERVICE_UNAVAILABLE",
            "INSTALLATION_UNAVAILABLE",
            "installation 'notes' is unavailable: this space does not serve it yet",
        ],
    ]);

    // suspend it and refuse its callers at once, as only enabling it serves it again
    await installer.installation.update({
        scope: ids.space,
        id: notes.id,
        requestId: RequestId.create(fixture.machine.environment),
        revision: (await readInstalled(database, notes.id)).revision,
        status: "suspended",
    });
    expect(await served(ids.space, "notes")).toEqual([
        "PRECONDITION_FAILED",
        "INSTALLATION_BLOCKED",
        "cannot reach installation 'notes': it is suspended",
    ]);

    // reset the space's stack to before its first pass, and answer a name it may still create to wait until it applies
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ conditions: {} })
                .where(
                    and(
                        eq(installation.table.scope, ids.space),
                        eq(installation.table.name, STACK_INSTALLATION),
                    ),
                ),
    );
    expect(await served(ids.space, "tasks")).toEqual([
        "SERVICE_UNAVAILABLE",
        "INSTALLATION_UNAVAILABLE",
        "installation 'tasks' is unavailable: this space does not serve it yet",
    ]);
});

test("hold a request for an installation until it serves, and refuse one the space does not have at once", async () => {
    // install notes without a build, then wait for it to serve as a request does
    const fixture = await TestSpace.open();
    await fixture.serve();
    const space = await fixture.space;
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(await unbuilt(fixture)),
        name: "notes",
    });
    const read = () =>
        Installation.requireServed(fixture.database, { spaceId: ids.space, installation: "notes" });
    const held = space.waitServing(read, new AbortController().signal);

    // deploy its build while the request waits
    const recorded = await fixture.recordBuild(
        await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto),
    );
    await fixture
        .as(fixture.owner.subject)
        .client(stackPackage)
        .installation.reconcile({
            scope: ids.space,
            id: notes.id,
            requestId: RequestId.create(fixture.machine.environment),
            build: recorded.build,
        });
    await fixture.simulation.idle();

    // answer the held request with notes, and refuse an installation the space does not have without holding
    const waited = await refusal(
        space.waitServing(
            () =>
                Installation.requireServed(fixture.database, {
                    spaceId: ids.space,
                    installation: "tasks",
                }),
            new AbortController().signal,
        ),
    );
    expect([(await held).id, waited]).toEqual([
        notes.id,
        [
            "NOT_FOUND",
            "INSTALLATION_NOT_FOUND",
            "installation 'tasks' served by this machine in this space not found",
        ],
    ]);
});

test("find the one installation serving a package, else ask to wait or refuse", async () => {
    // install notes, then another package built from its checkout, which serves the same package as notes does
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.serve();
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const install = async (name: string, packageId: PackageId) => {
        const recorded = await fixture.recordBuild(
            await fixtureBuild(packageId, "2026.9.0", fixture.environment.crypto),
        );
        const created = await installer.installation.create({
            scope: ids.space,
            requestId: RequestId.create(fixture.machine.environment),
            packageId,
            ...following(await unbuilt(fixture)),
            name,
        });
        await fixture
            .as(fixture.owner.subject)
            .client(stackPackage)
            .installation.reconcile({
                scope: ids.space,
                id: created.id,
                requestId: RequestId.create(fixture.machine.environment),
                build: recorded.build,
            });
        await fixture.simulation.idle();

        return created;
    };
    const serving = (
        spaceId: Identifier<"space">,
        packageId: PackageId,
        packages: readonly PackageId[] = [packageId],
    ) =>
        refusal(
            Installation.requireServing(database, { spaceId, packageId, packages }).then(
                ({ id: installationId }) => installationId,
            ),
        );
    const notes = await install("notes", ids.package);
    const alone = await Installation.requireServing(database, {
        spaceId: ids.space,
        packageId: ids.package,
        packages: [ids.package],
    });
    const unknown = await serving(ids.space, OTHER_PACKAGE);
    const pending = await serving(schema.identifier("space").parse(UNAPPLIED_SPACE), ids.package);
    await install("other", OTHER_PACKAGE);
    const twice = await serving(ids.space, ids.package, [ids.package, OTHER_PACKAGE]);

    // find notes alone, refuse an unknown package, ask to wait before the stack applies, and refuse two
    expect([alone.id, unknown, pending, twice]).toEqual([
        notes.id,
        [
            "NOT_FOUND",
            "INSTALLATION_NOT_FOUND",
            `installation serving package '${OTHER_PACKAGE}' in this space not found`,
        ],
        [
            "SERVICE_UNAVAILABLE",
            "INSTALLATION_UNAVAILABLE",
            `installation serving package '${ids.package}' is unavailable: this space does not serve it yet`,
        ],
        [
            "CONFLICT",
            "INSTALLATION_CONFLICT",
            `cannot route package '${ids.package}': installations 'notes', 'other' each serve it`,
        ],
    ]);
});

/** The region a space must place for the regional package, which declares no default. */
const region = defineSetting({
    name: "region",
    title: t`Region`,
    description: t`The region the space serves.`,
    schema: schema.string().min(1),
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});

test("refuse deploying a build needing a setting the space leaves unset", async () => {
    // record a checkout build of a package declaring a required region
    const fixture = await TestSpace.open();
    await fixture.serve();
    const installer = fixture.as(fixture.owner.subject).client(stackPackage);
    const regional = PackageVersion.parse({
        id: ids.package,
        name: "@example/notes",
        version: "2026.9.0",
    });
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
        ...following(await unbuilt(fixture)),
        name: "notes",
    });

    // refuse its submission while the space places no region
    const submitted = await refusal(
        deployBuild(fixture, notes.id, await fixture.recordBuild(declaring)),
    );
    expect([submitted, (await readInstalled(fixture.database, notes.id)).targetRevisionId]).toEqual(
        [
            [
                "PRECONDITION_FAILED",
                "SETTING_BLOCKED",
                `cannot read setting '${ids.package}/region': it has no default and is not set`,
            ],
            null,
        ],
    );
});
