import {
    installation,
    Binding,
    type Installation,
    type StackEvaluation,
    type Submission,
} from "@destack/stack/object";
import { checkout } from "../computer/checkout/object/index.ts";
import { describedTables, testIdentity } from "@destack/object/test";
import { KNOWN_TYPES } from "@destack/stack/stack";
import type { Environment } from "@destack/environment";
import {
    InstallationDefinition,
    install,
    StackDefinition,
    defineStack,
} from "@destack/stack/declare";
import objectPackage from "@destack/object/package";
import {
    BindingTarget,
    role,
    PACKAGE_KIND,
    Package,
    SERVER_EXPORT,
    Observation,
} from "@destack/object";
import { STACKS } from "@destack/account/test";
import { space } from "@destack/account/object";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { and, defineDatabase, eq, isNotNull } from "@destack/db";
import { databaseProvider } from "@destack/db/database";
import { BunSqliteDatabaseHost } from "@destack/db/bun";
import { pathToFileURL } from "node:url";
import { implementPackage, ObjectStore } from "@destack/object/server";
import { Plan } from "@destack/resource";
import {
    Digest,
    Identifier,
    type JsonObject,
    present,
    schema,
    type JsonValue,
} from "@destack/schema";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { describeObject, describePackage } from "@destack/object/inspect";
import { type DependencyRelease, PackageVersion } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { defineBucket } from "@destack/bucket";
import stackPackage from "@destack/stack/package";
import * as object from "@destack/stack/object";
import {
    evaluateStack,
    fixtureOutputs,
    fixtureBuild,
    imageOf,
    readStack,
    TestSpace,
    ids,
} from "../test/index.ts";
import { gauge, meter } from "../test/schema.ts";
import { RequestId, ObjectReference } from "@destack/sync";
import { Simulation } from "@destack/environment/simulation";
import { testWorldOptions } from "@destack/test/world";

/** A database a stack declares for its own use, in addition to the space's main database. */
const archiveDatabase = defineDatabase({ name: "archive", tables: [] });

/** A bucket a stack declares for its installations' files. */
const filesBucket = defineBucket({ name: "files", spec: {} });

/** A stack with an archive database and a files bucket. */
const personal = defineStack({
    types: [],
    databases: { archive: archiveDatabase.provision({ retention: { within: { days: 30 } } }) },
    buckets: { files: filesBucket.provision({ retention: { within: { days: 30 } } }) },
});

/** Submit a build declaring a stack to a space, apply it, and reconcile its installations. */
async function submit(
    fixture: TestSpace,
    store: ObjectStore,
    stack: Installation,
    submission: Omit<Submission, "build" | "stack"> & {
        /** The export, its arguments and the definition the stack's build declares there. */
        readonly stack: Omit<StackEvaluation, "settings" | "routes">;
    },
): Promise<Installation> {
    // record a build of the stack declaring the definition, and submit it for the space to evaluate
    const { export: exported, parameters, definition } = submission.stack;
    const recorded = await fixture.recordStack(exported, definition);
    await store.mutate(store.as(installation), {
        id: RequestId.create(fixture.machine.environment),
        calls: [
            installation.call("update", {
                ...installation.target(stack),
                ...ObjectReference.columns(
                    "version",
                    object.build.reference(stack.scope, recorded.build),
                ),
                directory: null,
                export: exported,
                parameters,
            }),
        ],
    });

    // wait until the machine applies it and the installations it changed
    await fixture.simulation.idle();

    return readStack(store.database);
}

/** Describe a submission of a stack definition from the checkout. */
function submissionOf(definition: JsonObject) {
    return {
        version: checkout.reference(ids.machine, ids.checkout),
        directory: ".",
        stack: { export: "personal", parameters: {}, definition },
    };
}

/** Evaluate the personal stack. */
function evaluatePersonal(): JsonObject {
    return evaluateStack({ personal }, "personal", {});
}

/** Serve a space and read its store. */
async function serve(fixture: TestSpace) {
    await fixture.serve();

    return await fixture.store();
}

/** The server of gauges in the space. */
const gauges = implementPackage(Package.of(gauge.package, { objects: [gauge] }), {
    objects: [gauge],
});

/** The server of meters in the space. */
const meters = implementPackage(Package.of(meter.package, { objects: [meter] }), {
    objects: [meter],
});

test("apply a stack submission, reapply it unchanged, and retire removed records once approved", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;

    // register a space without a stack
    const definition = evaluatePersonal();
    const stack = await readStack(database);
    const submission = {
        version: checkout.reference(ids.machine, ids.checkout),
        directory: ".",
        stack: { export: "personal", parameters: {}, definition },
    };

    // apply the submission and create every declared record once, at a fixed time
    const now = fixture.machine.clock.now();
    const store = await serve(fixture);
    const first = await submit(fixture, store, stack, submission);
    expect([first.appliedRevisionId, first.conditions["Ready"]?.reason]).toEqual([
        first.targetRevisionId,
        "Applied",
    ]);
    const databases = await database
        .select({ name: object.database.table.name, manager: object.database.table.managerName })
        .from(object.database.table)
        .orderBy(object.database.table.name);
    expect(databases).toEqual([
        { name: "archive", manager: "archive" },
        { name: "main", manager: null },
    ]);

    // submit the same build again without changes onto the same revision
    const again = await submit(fixture, store, stack, submission);
    expect(again.appliedRevisionId).toBe(first.appliedRevisionId);

    // remove a resource from the definition and request its deletion
    const { archive: _archive, ...remaining } = schema.jsonObject().parse(definition["databases"]);
    const removal = {
        ...submission,
        stack: {
            ...submission.stack,
            definition: { ...definition, databases: remaining },
        },
    };
    const waiting = await submit(fixture, store, stack, removal);
    expect([waiting.conditions["Ready"]?.reason, waiting.plan]).toEqual([
        "AwaitingApproval",
        {
            steps: [
                {
                    action: "delete",
                    target: "database/archive",
                    risk: "destructive",
                    detail: "retire",
                },
            ],
        },
    ]);

    // approve the destructive plan and retire the resource
    const digest = await Plan.digest(
        present(waiting.plan, "the waiting plan"),
        fixture.environment.crypto,
    );
    await store.mutate(store.as(installation), {
        id: RequestId.create(fixture.machine.environment),
        calls: [
            installation.call("approve", {
                ...installation.target({ scope: waiting.scope, id: waiting.id }),
                plan: digest,
            }),
        ],
    });
    await fixture.simulation.idle();
    const [retired] = await database
        .select()
        .from(object.database.table)
        .where(eq(object.database.table.name, "archive"));
    expect(present(retired, "the retired database").deletedAt).toBe(now);
});

test("apply a waiting stack again once its resources turn ready, without waking itself while it waits", async () => {
    // link a release of notes declaring gauges, which the space serves once its stack installs it on the host
    const described = [
        {
            kind: "object",
            kindPackageId: objectPackage.id,
            name: "gauge",
            description: describeObject(gauge),
        },
    ];
    const release = await MemoryBuild.of(
        PackageVersion.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        described,
        testHost().crypto,
        {
            outputs: fixtureOutputs({}),
            tables: describedTables(described, ids.package, KNOWN_TYPES),
        },
    );

    // hold the provisioning of the stack's archive database, as a provider out of disk does
    let isHeld = true;
    const fixture = await TestSpace.open({
        servers: [gauges],
        linked: [release],
        resources: (directory, environment) => {
            const host = new BunSqliteDatabaseHost(pathToFileURL(`${directory}/`), environment);
            const base = databaseProvider(host, object.database);
            const held: typeof base = {
                ...base,
                provision: {
                    isPure: false,
                    provision: async (record) => {
                        if (isHeld) {
                            throw new Error("the disk is full");
                        }

                        return base.provision.provision(record);
                    },
                    destroy: (record) => base.provision.destroy(record),
                },
            };

            return { providers: [held], connectors: [] };
        },
    });
    const { database } = fixture;

    // install notes through the stack, then declare a gauge waiting for the archive database
    const notes = {
        packageId: ids.package,
        version: { kind: "host" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        isolation: "host",
        tags: {},
    };
    const installing = { ...evaluatePersonal(), installations: { notes } };
    const store = await serve(fixture);
    const stack = await submit(fixture, store, await readStack(database), submissionOf(installing));
    const declaring = submissionOf({ ...installing, gauges: { main: {} } });
    const applied = await submit(fixture, store, stack, declaring);
    expect(applied.conditions["Ready"]?.reason).toBe("WaitingForResources");

    // apply it again while it waits without writing
    const position = await database.log.position();
    await fixture.simulation.idle();
    expect(await database.log.position()).toEqual(position);

    // apply the waiting stack once its provider provisions the database
    isHeld = false;
    await fixture.simulation.waitUntil(
        async () => (await readStack(database)).conditions["Ready"]?.reason === "Applied",
    );
    const installed = await database
        .select({
            name: installation.table.name,
            manager: installation.table.managerInstallationId,
        })
        .from(installation.table)
        .where(and(eq(installation.table.scope, ids.space), isNotNull(installation.table.name)))
        .orderBy(installation.table.name);
    const managed = await database.select({ name: gauge.table.managerName }).from(gauge.table);
    const stacked = await readStack(database);
    expect([stacked.conditions["Ready"]?.reason, installed, managed]).toEqual([
        "Applied",
        [
            { name: "notes", manager: stacked.id },
            { name: "stack", manager: null },
        ],
        [{ name: "main" }],
    ]);
});

test("leave an unchanged declared installation unwritten when its stack applies again", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;

    // apply a stack declaring notes
    const notes = {
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
    };
    const store = await serve(fixture);
    const stack = await readStack(database);
    await submit(fixture, store, stack, {
        version: checkout.reference(ids.machine, ids.checkout),
        directory: ".",
        stack: {
            export: "personal",
            parameters: {},
            definition: { ...evaluatePersonal(), installations: { notes } },
        },
    });

    // read the revisions of the notes installation and its rollout
    const revisions = async () => {
        const [installed] = await database
            .select({ id: installation.table.id, revision: installation.table.revision })
            .from(installation.table)
            .where(eq(installation.table.name, "notes"));
        const found = present(installed, "the notes installation");
        const [followed] = await database
            .select({ revision: installation.table.revision })
            .from(installation.table)
            .where(eq(installation.table.id, found.id));

        return [found.revision, present(followed, "the notes rollout").revision];
    };
    const before = await revisions();

    // apply the same definition again, as after a restart that lost the applied revision
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(installation.table)
                .set({ appliedRevisionId: null })
                .where(eq(installation.table.id, stack.id)),
    );
    await fixture.simulation.idle();

    // read the installation's fields joined in, so the declaration compares equal and writes nothing
    expect([(await readStack(database)).conditions["Ready"]?.reason, await revisions()]).toEqual([
        "Applied",
        before,
    ]);
});

test("run a stack's host installation at the release the machine links, as it version the host's", async () => {
    // link a newer release of the fixture package, as a machine shipping it does
    const linked = await fixtureBuild(ids.package, "2026.10.0", testHost().crypto);
    const fixture = await TestSpace.open({ linked: [linked] });
    const { database } = fixture;

    // apply a stack declaring the package on the host, following the host's release
    const notes = {
        packageId: ids.package,
        version: { kind: "host" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        isolation: "host",
        tags: {},
    };
    const store = await serve(fixture);
    await submit(fixture, store, await readStack(database), {
        version: checkout.reference(ids.machine, ids.checkout),
        directory: ".",
        stack: {
            export: "personal",
            parameters: {},
            definition: { ...evaluatePersonal(), installations: { notes } },
        },
    });

    // read the release the installation's revision runs
    const [revised] = await database
        .select({ image: object.build.table.image, source: object.build.table.source })
        .from(installation.table)
        .innerJoin(
            object.installationRevision.table,
            eq(object.installationRevision.table.id, installation.table.targetRevisionId),
        )
        .innerJoin(
            object.build.table,
            eq(object.build.table.id, object.installationRevision.table.buildId),
        )
        .where(eq(installation.table.name, "notes"));
    expect(revised).toEqual({
        image: await fixture.machine.resolve(ids.package, "2026.10.0"),
        source: {
            kind: "release",
            package: ids.package,
            version: "2026.10.0",
            image: await fixture.machine.resolve(ids.package, "2026.10.0"),
        },
    });
});

test("run a host installation at a restarted machine's linked release, then revise it there", async () => {
    // apply a stack declaring the package on the host while the machine links 2026.10.0
    const first = await fixtureBuild(ids.package, "2026.10.0", testHost().crypto);
    const fixture = await TestSpace.open({ linked: [first] });
    const { database } = fixture;
    const notes = {
        packageId: ids.package,
        version: { kind: "host" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        isolation: "host",
        tags: {},
    };
    const store = await serve(fixture);
    await submit(fixture, store, await readStack(database), {
        version: checkout.reference(ids.machine, ids.checkout),
        directory: ".",
        stack: {
            export: "personal",
            parameters: {},
            definition: { ...evaluatePersonal(), installations: { notes } },
        },
    });

    // restart the machine linking 2026.10.1, and open the installed set as a start does
    const next = await fixtureBuild(ids.package, "2026.10.1", testHost().crypto);
    await fixture.link(next);
    const [older, newer] = [
        await imageOf(first, testHost().crypto),
        await imageOf(next, testHost().crypto),
    ];
    const revised = async () =>
        (
            await database
                .select({ image: object.build.table.image })
                .from(installation.table)
                .innerJoin(
                    object.installationRevision.table,
                    eq(object.installationRevision.table.id, installation.table.targetRevisionId),
                )
                .innerJoin(
                    object.build.table,
                    eq(object.build.table.id, object.installationRevision.table.buildId),
                )
                .where(eq(installation.table.name, "notes"))
        )[0]?.image;
    const recorded = await revised();
    const opened = (await fixture.installed()).find(
        (entry) => entry.installation.name === "notes",
    )?.image;

    // restart the machine to revise onto the linked release
    await fixture.restart({});
    await fixture.simulation.idle();
    expect({ recorded, opened, moved: await revised() }).toEqual({
        recorded: older,
        opened: newer,
        moved: newer,
    });
});

test("hold a host installation of a package this machine does not provide", async () => {
    // declare the package on the host at a release, which only the host's release may be
    const fixture = await TestSpace.open();
    const { database } = fixture;
    const notes = {
        packageId: ids.package,
        version: { kind: "host" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        isolation: "host",
        tags: {},
    };
    const pinned = InstallationDefinition.safeParse({
        ...notes,
        version: { kind: "release", version: "2026.9.0" },
    });

    // apply a stack following the host's release of a package this machine provides none of
    const store = await serve(fixture);
    await submit(fixture, store, await readStack(database), {
        version: checkout.reference(ids.machine, ids.checkout),
        directory: ".",
        stack: {
            export: "personal",
            parameters: {},
            definition: { ...evaluatePersonal(), installations: { notes } },
        },
    });

    // read the stack's condition and the installation's deployments, holding it with none
    fixture.expectRefusals([
        [
            "PRECONDITION_FAILED",
            "TARGET_BLOCKED",
            `cannot follow the host's release of '${ids.package}': this machine provides none`,
        ],
    ]);
    const [installed] = await database
        .select({ id: installation.table.id, conditions: installation.table.conditions })
        .from(installation.table)
        .where(eq(installation.table.name, "notes"));
    const found = present(installed, "the notes installation");
    const deployments = await database
        .select({ id: object.deployment.table.id })
        .from(object.deployment.table)
        .where(eq(object.deployment.table.installationId, found.id));
    expect({
        pinned: pinned.error?.issues.map((issue) => issue.message),
        sandboxed: InstallationDefinition.safeParse({ ...notes, isolation: "sandbox" }).success,
        synced: found.conditions["Synced"]?.message,
        deployments,
    }).toEqual({
        pinned: ["a host installation runs the host's release"],
        sandboxed: true,
        synced: `@destack/stack TARGET_BLOCKED: cannot follow the host's release of '${ids.package}': this machine provides none`,
        deployments: [],
    });
});

test("replace an installation's owned bucket binding with its stack's, releasing it", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;

    // administer a space with a stack that declares a role and installs notes, whose deploy owns a bucket for its files
    const notes = {
        packageId: ids.package,
        version: { kind: "release", version: "2026.9.2" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        isolation: "sandbox",
        tags: {},
    };
    const stack = await readStack(database);
    const submission = {
        version: checkout.reference(ids.machine, ids.checkout),
        directory: ".",
        stack: {
            export: "personal",
            parameters: {},
            definition: {
                ...evaluatePersonal(),
                roles: { reader: { description: "Read", permissions: [] } },
                installations: { notes },
            },
        },
    };
    const store = await serve(fixture);
    await submit(fixture, store, stack, submission);

    // bind the declaration in the stack to replace the owned binding and release its bucket
    const bindings = {
        [ids.package]: { files: { target: { type: "bucket", name: "files" } } },
    };
    await submit(fixture, store, stack, {
        ...submission,
        stack: {
            ...submission.stack,
            definition: {
                ...submission.stack.definition,
                installations: { notes: { ...notes, bindings } },
            },
        },
    });
    await fixture.simulation.idle();
    const [files] = await database
        .select()
        .from(object.bucket.table)
        .where(eq(object.bucket.table.name, "files"));
    const released = await database
        .select({ name: object.bucket.table.name })
        .from(object.bucket.table)
        .where(isNotNull(object.bucket.table.deletedAt));
    const [named] = await database
        .select({ id: installation.table.id, scope: installation.table.scope })
        .from(installation.table)
        .where(eq(installation.table.name, "notes"));
    const installed = present(named, "the notes installation");
    const bound = (
        await Binding.targeted(
            database.snapshot(),
            schema.identifier("space").parse(installed.scope),
            installed.id,
        )
    ).map((entry) => ({ name: entry.name, target: entry.target }));
    // bind the installation itself, which the stack's binding selects for it
    expect({ bound, released }).toEqual({
        bound: [{ name: "files", target: present(files, "the files bucket").id }],
        released: [{ name: "notes.files" }],
    });
});

test("bind a declaration to another space's installation by name, refusing a pin", async () => {
    const fixture = await TestSpace.open();
    const stack = await readStack(fixture.database);
    const store = await serve(fixture);

    // submit notes with its tasks dependency bound to an installation of another space
    const example = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-00000000003a");
    const board = "board";
    const applied = async (target: JsonValue, version?: number) => {
        const tasks = { target, ...(version === undefined ? {} : { version }) };
        const notes = {
            packageId: ids.package,
            version: { kind: "release", version: "2026.9.3" },
            status: "enabled",
            name: "notes",
            bindings: { [ids.package]: { tasks } },
            compute: {},
            capabilities: [],
            directories: [],
            isolation: "sandbox",
            tags: {},
        };

        return await submit(fixture, store, stack, {
            version: checkout.reference(ids.machine, ids.checkout),
            directory: ".",
            stack: {
                export: "personal",
                parameters: {},
                definition: { ...evaluatePersonal(), installations: { notes } },
            },
        });
    };
    const bound = progressOf(
        await applied({ type: "installation", name: board, spaceId: example }),
    );
    const revisions = await fixture.database.select().from(object.installationRevision.table);
    const rows = revisions.flatMap((revision) =>
        revision.bindings.map((entry) => ({
            name: entry.name,
            target: entry.target,
            targetSpaceId: entry.targetSpaceId,
        })),
    );

    // keep the binding with the other space and block pinning or rebinding it
    expect({
        bound,
        rows,
        pinned: await TestSpace.refusals(
            applied({ type: "installation", name: board, spaceId: example }, 2),
        ),
        database: BindingTarget.safeParse({
            type: "database",
            name: board,
            spaceId: example,
        }).error?.issues.map((issue) => [issue.code, issue.message]),
    }).toEqual({
        bound: ["current", "every declaration applied"],
        rows: [{ name: "tasks", target: board, targetSpaceId: example }],
        pinned: [
            [
                "BAD_REQUEST",
                "DEFINITION_INVALID",
                `binding of '${board}' is invalid: it must select an installation of another space by name, without a version`,
            ],
        ],
        database: [["unrecognized_keys", 'Unrecognized key: "spaceId"']],
    });
});

test("block a stack relating a role or installation of another space by identifier", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;

    // register the space and a role in another space of the account
    const now = fixture.machine.clock.now();
    const other = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-00000000000a");
    await database.transaction(
        async (transaction) =>
            await transaction.insert(space.table).values({
                id: other,
                scope: ids.account,
                name: "shared",
                stack: STACKS.space,
                createdAt: now,
                updatedAt: now,
            }),
    );
    const foreign = schema.identifier("role").parse("role-01996ab0-0000-7000-8000-00000000000b");
    await database.transaction(
        async (transaction) =>
            await transaction.insert(role.table).values({
                id: foreign,
                scope: other,
                name: "editor",
                description: "Edit",
                createdAt: now,
                updatedAt: now,
            }),
    );
    const stack = await readStack(database);
    const store = await serve(fixture);

    // submit a stack binding the foreign role and one relating a foreign installation
    const blocked = (grant: JsonValue) =>
        TestSpace.refusals(
            submit(fixture, store, stack, {
                version: checkout.reference(ids.machine, ids.checkout),
                directory: ".",
                stack: {
                    export: "personal",
                    parameters: {},
                    definition: {
                        roles: { reader: { description: "Read", permissions: [] } },
                        relationships: { grant },
                    },
                },
            }),
        );
    const installationId = "installation-01996ab0-0000-7000-8000-00000000000c";
    const user = "user-01996ab0-0000-7000-8000-00000000000d";
    expect([
        await blocked({ subject: { id: user }, role: { id: foreign } }),
        await blocked({ subject: { id: installationId }, role: "reader" }),
    ]).toEqual([
        [["BAD_REQUEST", "DECLARATION_INVALID", `role '${foreign}' is invalid: it is unknown`]],
        [
            [
                "BAD_REQUEST",
                "DECLARATION_INVALID",
                `installation '${installationId}' is invalid: it is unknown`,
            ],
        ],
    ]);
});

test("install a package and declare records of its type in one approved plan, applied once the store serves the type", async () => {
    const fixture = await TestSpace.open({ servers: [meters] });
    const { database } = fixture;

    // release notes declaring meters, and require approval of every plan in the space
    const described = [
        {
            kind: "object",
            kindPackageId: objectPackage.id,
            name: "meter",
            description: describeObject(meter),
        },
    ];
    const release = await MemoryBuild.of(
        PackageVersion.parse({ id: ids.package, name: "@example/notes", version: "2027.1.0" }),
        described,
        fixture.environment.crypto,
        {
            outputs: fixtureOutputs({}),
            tables: describedTables(described, ids.package, KNOWN_TYPES),
        },
    );
    await fixture.publish(release);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(space.table)
                .set({ approval: "safe" })
                .where(eq(space.table.id, ids.space)),
    );

    // submit a stack installing notes and declaring a meter
    const notes = {
        packageId: ids.package,
        version: { kind: "release", version: "2027.1.0" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        isolation: "sandbox",
        tags: {},
    };
    const definition = { installations: { notes }, meters: { main: {} } };
    const stack = await readStack(database);
    const store = await serve(fixture);
    const recorded = await fixture.recordStack("personal", definition);
    await store.mutate(store.as(installation), {
        id: RequestId.create(fixture.machine.environment),
        calls: [
            installation.call("update", {
                ...installation.target(stack),
                ...ObjectReference.columns(
                    "version",
                    object.build.reference(stack.scope, recorded.build),
                ),
                directory: null,
                export: "personal",
                parameters: {},
            }),
        ],
    });

    // plan installing notes and declaring the meter as one plan waiting for approval
    await fixture.simulation.idle();
    const waiting = await readStack(database);
    expect([waiting.conditions["Ready"]?.reason, waiting.plan]).toEqual([
        "AwaitingApproval",
        {
            steps: [
                { action: "create", target: "installation/notes", risk: "safe", detail: "declare" },
                { action: "create", target: "meter/main", risk: "safe", detail: "declare" },
            ],
            deferred: "meters wait for the store to serve their types",
        },
    ]);

    // approve it, and install notes while the meter waits for the store to serve meters
    const digest = await Plan.digest(
        present(waiting.plan, "the waiting plan"),
        fixture.environment.crypto,
    );
    await store.mutate(store.as(installation), {
        id: RequestId.create(fixture.machine.environment),
        calls: [
            installation.call("approve", {
                ...installation.target({ scope: waiting.scope, id: waiting.id }),
                plan: digest,
            }),
        ],
    });
    await fixture.simulation.idle();
    const installed = await readStack(database);
    expect(installed.conditions["Ready"]?.message).toBe(
        "meters wait for the store to serve their types",
    );

    // approve the plan creating the meters' table, which notes waits on
    const owner = fixture.as(fixture.owner.subject).client(stackPackage);
    const target = { scope: ids.space, id: installed.id };
    const migrating = await owner.installation.plan(target);
    await owner.installation.approve({
        ...target,
        requestId: RequestId.create(fixture.machine.environment),
        plan: migrating.digest,
    });
    await fixture.simulation.idle();
    expect(migrating.plan.steps.map((step) => step.target)).toEqual([
        "database/main/table/example__notes__meter",
        "database/main/table/example__notes__meter/dependent/destack__access__relationship",
    ]);

    // link notes and restart the machine
    const image = await fixture.machine.resolve(ids.package, "2027.1.0");
    fixture.machine.link({ package: PackageManifest.package(release.manifest), image });
    await fixture.restart({});
    await fixture.simulation.idle();
    const applied = await readStack(database);
    const declared = await database.select({ name: meter.table.managerName }).from(meter.table);
    expect([applied.conditions["Ready"]?.reason, applied.appliedRevisionId, declared]).toEqual([
        "Applied",
        applied.targetRevisionId,
        [{ name: "main" }],
    ]);
});

test("declare a stack's installations as its document lists them, the space deriving what they require, and refuse a package outside the stack's image", async () => {
    // publish an installable dependency and a library, and two installable packages depending on them
    const world = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 1),
    }).environment("test");
    const vault = await fixtureRelease("@example/vault", "2026.10.0", true, [], world);
    const library = await fixtureRelease("@example/library", "2026.10.0", false, [], world);
    const notes = await fixtureRelease(
        "@example/notes",
        "2026.10.0",
        true,
        [vault, library],
        world,
    );
    const pages = await fixtureRelease("@example/pages", "2026.10.0", true, [vault], world);

    // declare notes alone, then notes beside pages, whose build the stack's image lacks
    const declare = async (installed: readonly FixtureRelease[]) =>
        StackDefinition.declared(
            await installingStack(installed, world),
            Object.values(entriesOf([vault, library, notes])),
            "space",
            { settings: {}, routes: {} },
        );
    const alone = StackDefinition.collections(await declare([notes]));
    const outside = await refusal(declare([notes, pages]));

    // keep notes alone, leaving its vault for the space to resolve, and refuse pages
    expect([Object.keys(alone.installations), outside]).toEqual([
        ["notes"],
        [
            "BAD_REQUEST",
            "DEFINITION_INVALID",
            "installation 'pages' of 'space' is invalid: it installs a package without a build in the stack's image, so add the package to the stack's dependencies",
        ],
    ]);
});

test("keep a stack's installation pins as declared, and run an unpinned one at its release in the stack's image", async () => {
    // publish notes depending on an installable vault, and declare notes following its latest tag beside vault
    const world = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 1),
    }).environment("test");
    const vault = await fixtureRelease("@example/vault", "2026.10.0", true, [], world);
    const notes = await fixtureRelease("@example/notes", "2026.10.0", true, [vault], world);
    const latest = { kind: "tag", tag: "latest" } as const;
    const stack = await installingStack([notes, vault], world, { notes: latest });

    // run the tag for notes, and vault at the release the stack's image has
    const { installations } = StackDefinition.collections(
        await StackDefinition.declared(stack, Object.values(entriesOf([vault, notes])), "space", {
            settings: {},
            routes: {},
        }),
    );
    expect([installations["notes"]?.version, installations["vault"]?.version]).toEqual([
        latest,
        { kind: "release", version: "2026.10.0" },
    ]);
});

/** A fixture release of a package, with its build. */
interface FixtureRelease {
    /** The registry format. */
    readonly kind: "destack";
    /** The released package. */
    readonly package: PackageVersion;
    /** The digest of its manifest. */
    readonly manifest: Digest;
    /** Its build. */
    readonly build: BuildReader;
}

/** Publish a fixture release of a new package or a given one, with the releases it depends on. */
async function fixtureRelease(
    name: string,
    version: string,
    isInstallable: boolean,
    dependencies: readonly FixtureRelease[],
    environment: Environment,
    id: string = Identifier.create("package", environment),
): Promise<FixtureRelease> {
    // describe the package as installable, serving a type stored in a space, else as a library
    const owner = PackageVersion.parse({ id, name, version });
    const declared = isInstallable
        ? [
              {
                  ...PACKAGE_KIND.identity,
                  name: "default",
                  description: describePackage({
                      ...Package.of(owner, { objects: [] }),
                      ...testIdentity,
                  }),
              },
              {
                  kind: "object",
                  kindPackageId: objectPackage.id,
                  name: "meter",
                  description: describeObject(meter),
              },
          ]
        : [];
    const outputs = fixtureOutputs({
        server: {
            runtime: "bun",
            emit: true,
            workloads: {},
            views: {},
            exports: isInstallable
                ? { [SERVER_EXPORT]: { path: "server/index.js", imports: [] } }
                : {},
        },
    });
    const build = await MemoryBuild.of(owner, declared, environment.crypto, {
        requires: requirementsOf(dependencies),
        outputs,
    });
    const manifest = await Digest.of(
        new TextEncoder().encode(`${name}@${version}`),
        environment.crypto,
    );

    return { kind: "destack" as const, package: owner, manifest, build };
}

/** Build a stack installing some releases, resolving them, some following a tag by name. */
function installingStack(
    installed: readonly FixtureRelease[],
    environment: Environment,
    tags: Readonly<Record<string, { readonly kind: "tag"; readonly tag: string }>> = {},
): Promise<BuildReader> {
    // declare the stack's package and an installation of each release
    const owner = PackageVersion.parse({
        id: Identifier.create("package", environment),
        name: "@example/stack",
        version: "2026.10.0",
    });
    const installations = Object.fromEntries(
        installed.map((each) => {
            const name = each.package.name.slice(each.package.name.lastIndexOf("/") + 1);
            const version = tags[name];
            const options = version === undefined ? {} : { version };

            return [name, install({ ...each.package, resources: {}, secrets: {} }, {}, options)];
        }),
    );

    return MemoryBuild.of(
        owner,
        [
            {
                kind: "stack",
                kindPackageId: stackPackage.id,
                name: "space",
                description: { installations },
            },
        ],
        environment.crypto,
        { requires: requirementsOf(installed) },
    );
}

/** Require releases at their versions, by package name. */
function requirementsOf(released: readonly FixtureRelease[]): Record<string, string> {
    return Object.fromEntries(released.map((each) => [each.package.name, each.package.version]));
}

/** Read releases as an image's entries, by package name. */
function entriesOf(released: readonly FixtureRelease[]): Record<string, DependencyRelease> {
    return Object.fromEntries(
        released.map(({ kind, package: dependency, manifest }) => [
            dependency.name,
            { kind, package: dependency, manifest },
        ]),
    );
}

test("refuse a declaration in the enclosing account from a space that is not the account's home", async () => {
    // apply a role declared in the account from an ordinary space of it
    const fixture = await TestSpace.open();
    const submitted = await fixture.submit([], {
        roles: { reviewer: { description: "Review notes", permissions: [], scope: "enclosing" } },
    });

    // refuse the document at its plan, before writing anything
    expect(submitted).toEqual([
        "BAD_REQUEST",
        "DECLARATION_INVALID",
        "role 'reviewer' is invalid: it is declared in the enclosing scope, which only its home's stack declares into",
    ]);
});

/** Read how far a stack is, with what it reports. */
function progressOf(submitted: Installation) {
    return [Observation.progress(submitted), Observation.message(submitted)];
}
