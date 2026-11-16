import { ObjectReference } from "@destack/sync";
import { checkout } from "../src/computer/checkout/object/index.ts";
import { TestRegistry } from "../src/test/build.ts";
import { Lock } from "@destack/package/manifest";
import { installation } from "@destack/object";
import objectPackage from "@destack/object/package";
import { Plan } from "@destack/resource";
import { expect, refusal, test } from "@destack/test";
import { schema, present, Digest, type Identifier, type JsonObject } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { CallError } from "@destack/object";
import { and, eq, Snapshot } from "@destack/db";
import { build, installationRevision, Rollout, rollout } from "@destack/stack/object";
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
    ids,
    insertRollout,
    TestSpace,
} from "../src/test/index.ts";
import { BuildStore } from "@destack/build/store";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import { RolloutTarget } from "../src/installation/target.ts";
import { t } from "@destack/locale";

/** A space whose stack this machine has not applied yet. */
const UNAPPLIED_SPACE = "space-01996ab0-0000-7000-8000-0000000000f4";

/** The checkout the stack's builds come from. */
const source = {
    kind: "checkout" as const,
    machine: ids.machine,
    checkout: schema.identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000008"),
    directory: "." as const,
};

/** What following the checkout writes on an installation or a submission. */
const followed = {
    follows: checkout.reference(source.machine, source.checkout),
    directory: source.directory,
};

/** Follow a release of a package by its version. */
function release(version: string) {
    return { follows: TestRegistry.row("release", version) };
}

/** Write what an installation follows as its columns. */
function following(target: { readonly follows: ObjectReference; readonly directory?: string }) {
    return {
        ...ObjectReference.columns("follows", target.follows),
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
        .rollout.deploy({
            spaceId: ids.space,
            id,
            requestId: RequestId.create(fixture.machine.environment),
            build: built.build,
        });
}

/** Follow another row with an installation, as the space's owner. */
async function refollow(
    fixture: TestSpace,
    id: Identifier<"installation">,
    target: { readonly follows: ObjectReference; readonly directory?: string },
    stack: { readonly export?: string; readonly parameters?: JsonObject } = {},
) {
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
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
    const follows = build.reference(ids.space, built.build);

    return refollow(fixture, id, { follows }, { export: "personal", parameters: {} });
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
                selection: { kind: "release", version: "2026.9.0" },
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
    await followStack(
        fixture,
        stack.installationId,
        await fixture.recordStack("personal", definition),
    );
    await fixture.simulation.idle();
    const applied = await readInstalled(fixture.database, stack.installationId);
    expect([
        applied.appliedRevisionId === null,
        applied.installation.conditions["Ready"]?.reason,
    ]).toEqual([false, "Applied"]);
    expect(await owner.rollout.plan({ spaceId: ids.space, id: stack.installationId })).toEqual({
        plan: { steps: [] },
        digest: await Plan.digest({ steps: [] }, fixture.environment.crypto),
        approval: null,
    });

    // apply the notes installation, then open a view its applied revision declares and refuse one it does not
    const listed = await Rollout.find(fixture.database, ids.space, "notes");
    const notes = present(listed, "the notes installation");
    await fixture.simulation.idle();
    const revised = await readInstalled(fixture.database, notes.installationId);
    expect(
        await owner.rollout.open({
            spaceId: ids.space,
            id: notes.installationId,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.installationId,
        revisionId: revised.targetRevisionId,
        lock: await fixture.registry.find(ids.package, "2026.9.0"),
        view: "home",
        description: {
            entrypoint: "view.js",
            stylesheets: [],
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            categories: [],
            audience: "person",
            packages: [],
            render: { mode: "client" },
        },
        path: "/",
        package: "@test/package",
        permissions: [],
        home: fixture.owner.home,
    });
    expect(
        await refusal(
            owner.rollout.open({
                spaceId: ids.space,
                id: notes.installationId,
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
        await followStack(
            fixture,
            stack.installationId,
            await fixture.recordStack("personal", definition),
        );

        // apply the revision, approving the plan it waits on first
        await fixture.simulation.idle();
        const read = () => readInstalled(fixture.database, stack.installationId);
        const isWaiting = (await read()).installation.plan !== null;
        if (isWaiting) {
            const { digest } = await owner.rollout.plan({
                spaceId: ids.space,
                id: stack.installationId,
            });
            await owner.rollout.approve({
                spaceId: ids.space,
                id: stack.installationId,
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
    const applied = await readInstalled(fixture.database, stack.installationId);
    expect([
        [first.isWaiting, second.isWaiting, third.isWaiting],
        applied.targetRevisionId,
        applied.appliedRevisionId,
        applied.installation.conditions["Ready"]?.reason,
    ]).toEqual([[false, true, true], first.revision, first.revision, "Applied"]);
});

test("install an application from a checkout by submitting its build, following its revision's views and declarations", async () => {
    const fixture = await TestSpace.open();
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(followed),
        name: "notes",
    });

    // submit the checkout's build, apply it, and open the view it mounts
    const reader = await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto);
    const recorded = await fixture.recordBuild(reader);
    await deployBuild(fixture, notes.id, recorded);
    await fixture.simulation.idle();
    const installed = await readInstalled(fixture.database, notes.id);
    expect(
        await owner.rollout.open({
            spaceId: ids.space,
            id: notes.id,
            view: "home",
            path: "/",
        }),
    ).toEqual({
        installationId: notes.id,
        revisionId: installed.targetRevisionId,
        lock: await Lock.digest(
            Lock.of(await BuildStore.digest(reader.manifest, fixture.environment.crypto)),
            fixture.environment.crypto,
        ),
        view: "home",
        description: {
            entrypoint: "view.js",
            stylesheets: [],
            sandbox: {},
            presents: [{ packageId: ids.package, type: "note", priority: "option" }],
            categories: [],
            audience: "person",
            packages: [],
            render: { mode: "client" },
        },
        path: "/",
        package: "@test/package",
        permissions: [],
        home: fixture.owner.home,
    });
    const declared = await owner.declaration.list({ buildId: recorded.build });
    const sorted = declared.items.toSorted((left, right) =>
        left.moniker.localeCompare(right.moniker),
    );
    expect(sorted.map(({ kind, name, description }) => [kind, name, description])).toEqual([
        ["command", "archive", ARCHIVE],
        ["relation", "editor", {}],
    ]);
});

test("refuse submitting a build that has not succeeded or that the space does not have", async () => {
    // record a pending build of the checkout
    const fixture = await TestSpace.open();
    const { database } = fixture;
    await fixture.serve();
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(followed),
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
                source: { ...source },
                outputs: ["bun"],
                createdAt: now,
                updatedAt: now,
            }),
    );

    // refuse the pending build and a build of no row
    const submit = (id: typeof pending) =>
        owner.rollout
            .deploy({
                spaceId: ids.space,
                id: notes.id,
                requestId: RequestId.create(fixture.machine.environment),
                build: id,
            })
            .catch((error: unknown) => error);
    expect([await submit(pending), await submit(missing)]).toEqual([
        new CallError("SERVICE_UNAVAILABLE", {
            reason: "BUILD_UNAVAILABLE",
            domain: "@destack/stack",
            defined: true,
            message: `build '${pending}' is unavailable: it is 'pending'`,
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
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(release("2026.9.0")),
        name: "notes",
    });
    const submit = (version: string) => refollow(fixture, notes.id, release(version));
    const read = () => readInstalled(fixture.database, notes.id);

    // apply the first release and wait on the upgrade to the last
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await fixture.simulation.idle();
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
    const { plan: waiting, digest } = await owner.rollout.plan({
        spaceId: ids.space,
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
        approved.installationId,
        applied.appliedRevisionId === applied.targetRevisionId,
        applied.installation.plan,
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
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(release("2026.9.0")),
        name: "notes",
    });
    const follow = (version: string) => refollow(fixture, notes.id, release(version));
    const read = () => readInstalled(fixture.database, notes.id);

    // refuse the fork's build at once
    const deploy = async () => {
        // record the targets bound, then deploy once the space serves the bound revision
        await fixture.simulation.idle();
    };
    await deploy();
    const deployed = (await read()).installation.conditions["Ready"]?.reason;
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
        [upgraded.installation.conditions["Ready"]?.reason, upgraded.installation.plan],
        [rolledBack.installation.conditions["Ready"]?.reason, rolledBack.installation.plan],
    ]).toEqual([
        "Deployed",
        [
            "BAD_REQUEST",
            "PACKAGE_INVALID",
            "build is invalid: it is of '@test/package', which 'notes' does not install",
        ],
        ["Deployed", null],
        ["Deployed", null],
    ]);
});

test("wait for a release no one has published under the selected tag, reporting it on the installation", async () => {
    // install the package following a tag its registry never published
    const fixture = await TestSpace.open();
    await fixture.serve();
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following({ follows: TestRegistry.row("tag", "beta") }),
        name: "notes",
    });

    // reconcile its rollout, which finds no release to revise
    await fixture.simulation.idle();
    const waiting = await readInstalled(fixture.database, notes.id);
    const ready = waiting.installation.conditions["Ready"];
    expect([waiting.targetRevisionId, ready?.status, ready?.reason, ready?.message]).toEqual([
        null,
        "false",
        "ReleaseNotPublished",
        "notes has no published release for what it follows",
    ]);
});

test("open the build each package's installation in a space follows, and none for a package the space does not install", async () => {
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
        await insertRollout(
            database,
            {
                id,
                scope: ids.space,
                packageId,
                name: `notes-${suffix}`,
                follows: TestRegistry.row("release", version),
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
                    .update(rollout.table)
                    .set({ targetRevisionId: revisionId })
                    .where(eq(rollout.table.installationId, id)),
        );
    };
    const { crypto } = fixture.machine.environment;
    await fixture.registry.release(await fixtureBuild(ids.stackPackage, "2026.10.0", crypto));
    await follow(ids.package, "2026.9.0", "e1");
    await follow(ids.stackPackage, "2026.10.0", "e2");

    // open each package's build, and none for a package the space does not install
    const space = await fixture.space;
    const version = (packageId: PackageId) =>
        RolloutTarget.reader(Snapshot.live(database), ids.space, packageId, undefined, {
            space,
            registry: fixture.registry,
            forge: null,
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
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const notes = await installer.installation.create({
        scope: ids.space,
        requestId: RequestId.create(fixture.machine.environment),
        packageId: ids.package,
        ...following(followed),
        name: "notes",
    });
    const recorded = await fixture.recordBuild(
        await fixtureBuild(ids.package, "2026.9.0", fixture.environment.crypto),
    );
    await fixture
        .as(fixture.owner.subject)
        .client(stackPackage)
        .rollout.deploy({
            spaceId: ids.space,
            id: notes.id,
            requestId: RequestId.create(fixture.machine.environment),
            build: recorded.build,
        });
    await fixture.simulation.idle();

    // read it by its identifier and its name, refuse another name, and answer a space before its first stack pass to wait
    const served = async (spaceId: string, named: string) =>
        refusal(
            Rollout.requireServed(database, { spaceId, installation: named }).then(
                ({ installationId }) => installationId,
            ),
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
        byId.installationId,
        byName.installationId,
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

    // suspend it and answer its callers to wait
    await installer.installation.update({
        scope: ids.space,
        id: notes.id,
        requestId: RequestId.create(fixture.machine.environment),
        revision: (await readInstalled(database, notes.id)).installation.revision,
        status: "suspended",
    });
    expect(await served(ids.space, "notes")).toEqual([
        "SERVICE_UNAVAILABLE",
        "INSTALLATION_UNAVAILABLE",
        "installation 'notes' is unavailable: this space does not serve it yet",
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
    const installer = fixture.as(fixture.owner.subject).client(objectPackage);
    const regional = PackageVersion.parse({
        id: ids.package,
        name: "@test/package",
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
        ...following(followed),
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
