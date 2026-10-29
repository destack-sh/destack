import type { Environment } from "@destack/environment";
import {
    declaredStack,
    install,
    StackBindingTarget,
    StackDefinition,
} from "../../declare/index.ts";
import objectPackage from "@destack/object/package";
import { role } from "@destack/object";
import { STACKS } from "@destack/account/test";
import { space } from "@destack/account/object";
import { type SpaceOptions, serveSpace } from "../index.ts";
import { pathToFileURL } from "node:url";
import { PACKAGE_DECLARATION, PackageHandle } from "@destack/object";
import { expect, refusal, test } from "@destack/test";
import {
    and,
    type DatabaseConnection,
    databaseProvider,
    defineDatabase,
    eq,
    isNotNull,
    ne,
    Snapshot,
} from "@destack/db";
import * as sqlite from "@destack/db/bun";
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
import type { BuildReader } from "@destack/package/manifest";
import { describeObject, describePackage } from "@destack/object/inspect";
import { Package, PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { defineBucket } from "@destack/bucket";
import { defineStack } from "../../declare/index.ts";
import stackPackage from "../../package.ts";
import * as object from "../../object/index.ts";
import { installation } from "@destack/object";
import {
    binding,
    Rollout,
    rollout,
    type Submission,
    type NamedInstallation,
} from "../../object/index.ts";
import {
    evaluateStack,
    fixtureBuild,
    fixtureOutputs,
    reconcile,
    settle,
    TestSpace,
    ids,
} from "../../test/index.ts";
import { gauge, meter } from "../../test/schema.ts";
import { RequestId } from "@destack/sync";
import { Simulation } from "@destack/environment/simulation";

/** A database a stack declares for its own use, in addition to the space's main database. */
const archiveDatabase = defineDatabase({ name: "archive", tables: [] });

/** A bucket a stack declares for its installations' files. */
const filesBucket = defineBucket({ name: "files", spec: {} });

/** A stack with an archive database and a files bucket. */
const personal = defineStack([], {
    databases: { archive: archiveDatabase.declare({ retention: { within: { days: 30 } } }) },
    buckets: { files: filesBucket.declare({ retention: { within: { days: 30 } } }) },
});

/** Read the space's one stack installation as it stands, the one its birth gives it. */
async function readStack(
    database: DatabaseConnection,
): Promise<Rollout & { readonly installation: NamedInstallation }> {
    const rows = await database.select().from(rollout.table).where(eq(rollout.table.kind, "stack"));
    const [stack] = await Rollout.read(Snapshot.live(database), rows);

    return present(stack, "the space's stack");
}

/** Submit a build of a manifest to a space's stack, apply it, and settle its installations. */
async function submit(
    fixture: TestSpace,
    store: Pick<ObjectStore, "database" | "as" | "mutate" | "controllers" | "environment">,
    stack: Rollout & { readonly installation: NamedInstallation },
    submission: Omit<Submission, "build">,
): Promise<Rollout & { readonly installation: NamedInstallation }> {
    // record the stack's build and the submission pinned as its evaluation is, as the system
    const before = await store.database.log.position();
    const build = await fixture.recordBuild(
        await fixtureBuild(stack.installation.packageId, "2026.9.0", fixture.environment.crypto),
    );
    const evaluation =
        submission.evaluation === undefined
            ? {}
            : {
                  evaluation: {
                      ...submission.evaluation,
                      definition: await fixture.resolve(submission.evaluation.definition),
                  },
              };
    await store.mutate(store.as(rollout), {
        id: RequestId.create(fixture.machine.environment),
        calls: [
            rollout.call("submit", {
                ...rollout.target(stack),
                ...submission,
                ...evaluation,
                build,
            }),
        ],
    });

    // apply it, then settle the installations it changed
    await reconcile(store, "rollout", { scope: ids.space, kind: "stack" });
    const installing = store.controllers().filter((each) => each.name === "rollout");
    await settle(store.database, installing, before);

    return readStack(store.database);
}

/** Evaluate the personal stack. */
function evaluatePersonal(): JsonObject {
    return evaluateStack({ personal }, "personal", {});
}

/** Serve a space's objects, with its databases as SQLite files in a scratch directory. */
async function serve(fixture: TestSpace, options: Partial<SpaceOptions> = {}) {
    const directory = await fixture.environment.files.makeTemporaryDirectory("destack-stack-");
    const databases = databaseProvider(
        new sqlite.BunSqliteDatabaseHost(pathToFileURL(`${directory}/`), fixture.environment),
        object.database,
    );

    return serveSpace(await fixture.options({ providers: [databases], ...options })).store;
}

/** The server of gauges in the space. */
const gauges = implementPackage(PackageHandle.of(gauge.package, { objects: { gauge } }), {
    objects: [gauge],
});

/** The server of meters in the space. */
const meters = implementPackage(PackageHandle.of(meter.package, { objects: { meter } }), {
    objects: [meter],
});

test("apply a stack submission, reapply it unchanged, and retire removed records once approved", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;

    // register a space without a stack
    const definition = evaluatePersonal();
    const stack = await readStack(database);
    const submission = {
        selection: {
            kind: "checkout" as const,
            machine: ids.machine,
            checkout: ids.checkout,
            directory: "." as const,
        },
        evaluation: { export: "personal", parameters: {}, definition },
    };

    // apply the submission and create every declared record once, at a fixed time
    const now = fixture.machine.clock.now();
    const store = await serve(fixture, { environment: fixture.machine.environment });
    const first = await submit(fixture, store, stack, submission);
    expect([first.appliedRevisionId, first.conditions["Ready"]?.reason]).toEqual([
        first.revisionId,
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
        evaluation: {
            ...submission.evaluation,
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
    await store.mutate(store.as(rollout), {
        id: RequestId.create(fixture.machine.environment),
        calls: [rollout.call("approve", { ...rollout.target(waiting), plan: digest })],
    });
    await reconcile(store, "rollout", { scope: ids.space, kind: "stack" });
    const [retired] = await database
        .select()
        .from(object.database.table)
        .where(eq(object.database.table.name, "archive"));
    expect(present(retired, "the retired database").deletedAt).toBe(now);
});

test("apply a waiting stack again once its resources turn ready, without waking itself while it waits", async () => {
    const fixture = await TestSpace.open({ packages: [{ objects: { gauge }, tables: [] }] });
    const { database } = fixture;

    // administer a space on this machine
    const now = fixture.machine.clock.now();

    // apply a stack with a gauge waiting for the database
    const notes = {
        kind: "package" as const,
        packageId: ids.package,
        selection: { kind: "release", version: "2026.9.0" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        tags: {},
    };
    const stack = await readStack(database);
    const submission = {
        selection: {
            kind: "checkout" as const,
            machine: ids.machine,
            checkout: ids.checkout,
            directory: "." as const,
        },
        evaluation: {
            export: "personal",
            parameters: {},
            definition: { ...evaluatePersonal(), installations: { notes }, gauges: { main: {} } },
        },
    };
    // reapply the space's main database, as its provider does while it applies a change
    const main = and(
        eq(object.database.table.scope, ids.space),
        eq(object.database.table.name, "main"),
    );
    const condition = (reason: string, status: "true" | "false") => ({
        Ready: {
            status,
            observedGeneration: 1,
            reason,
            message: `main is ${reason}`,
            lastTransitionAt: now,
        },
    });
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.database.table)
                .set({ conditions: condition("Applying", "false") })
                .where(main),
    );
    const store = await serve(fixture, { servers: [gauges] });
    const applied = await submit(fixture, store, stack, submission);
    expect(applied.conditions["Ready"]?.reason).toBe("WaitingForResources");

    // apply it again while it waits without writing
    const position = await database.log.position();
    await reconcile(store, "rollout", { scope: ids.space, kind: "stack" });
    expect(await database.log.position()).toEqual(position);

    // apply the waiting stack once its database turns ready
    const ready = await database.log.position();
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(object.database.table)
                .set({ conditions: condition("Applied", "true") })
                .where(main),
    );
    await settle(
        database,
        store.controllers().filter((each) => each.name === "rollout"),
        ready,
    );
    const installed = await database
        .select({ name: installation.table.name, origin: installation.table.origin })
        .from(installation.table)
        .where(and(eq(installation.table.scope, ids.space), ne(installation.table.origin, "base")))
        .orderBy(installation.table.name);
    const managed = await database.select({ name: gauge.table.managerName }).from(gauge.table);
    expect([(await readStack(database)).conditions["Ready"]?.reason, installed, managed]).toEqual([
        "Applied",
        [
            { name: "notes", origin: "declared" },
            { name: "stack", origin: "installed" },
        ],
        [{ name: "main" }],
    ]);
});

test("leave a declared installation whose fields are unchanged as it is when its stack applies it again", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;

    // apply a stack declaring notes
    const notes = {
        kind: "package" as const,
        packageId: ids.package,
        selection: { kind: "release", version: "2026.9.0" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        tags: {},
    };
    const store = await serve(fixture);
    await submit(fixture, store, await readStack(database), {
        selection: {
            kind: "checkout" as const,
            machine: ids.machine,
            checkout: ids.checkout,
            directory: "." as const,
        },
        evaluation: {
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
            .select({ revision: rollout.table.revision })
            .from(rollout.table)
            .where(eq(rollout.table.id, found.id));

        return [found.revision, present(followed, "the notes rollout").revision];
    };
    const before = await revisions();

    // apply the same definition again, as after a restart that lost the applied revision
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(rollout.table)
                .set({ appliedRevisionId: null })
                .where(eq(rollout.table.kind, "stack")),
    );
    await reconcile(store, "rollout", { scope: ids.space, kind: "stack" });

    // read the installation's fields joined in, so the declaration compares equal and writes nothing
    expect([(await readStack(database)).conditions["Ready"]?.reason, await revisions()]).toEqual([
        "Applied",
        before,
    ]);
});

test("replace an installation's own binding once its stack binds the declaration as a bucket next to its roles, releasing its previously owned bucket", async () => {
    const fixture = await TestSpace.open();
    const { database } = fixture;

    // administer a space with a stack that declares a role and installs notes, whose deploy owns a bucket for its files
    const notes = {
        kind: "package" as const,
        packageId: ids.package,
        selection: { kind: "release", version: "2026.9.2" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        tags: {},
    };
    const stack = await readStack(database);
    const submission = {
        selection: {
            kind: "checkout" as const,
            machine: ids.machine,
            checkout: ids.checkout,
            directory: "." as const,
        },
        evaluation: {
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
    const before = await database.log.position();
    const bindings = {
        [ids.package]: { files: { target: { type: "bucket", name: "files" } } },
    };
    await submit(fixture, store, stack, {
        ...submission,
        evaluation: {
            ...submission.evaluation,
            definition: {
                ...submission.evaluation.definition,
                installations: { notes: { ...notes, bindings } },
            },
        },
    });
    await settle(
        database,
        store.controllers().filter((each) => each.name === "rollout"),
        before,
    );
    const [files] = await database
        .select()
        .from(object.bucket.table)
        .where(eq(object.bucket.table.name, "files"));
    const released = await database
        .select({ name: object.bucket.table.name })
        .from(object.bucket.table)
        .where(isNotNull(object.bucket.table.deletedAt));
    const bound = await database
        .select({
            name: binding.table.name,
            target: binding.table.target,
            manager: binding.table.managerInstallationId,
        })
        .from(binding.table);
    // bind the installation itself, which the stack's binding selects for it
    expect({ bound, released }).toEqual({
        bound: [{ name: "files", target: present(files, "the files bucket").id, manager: null }],
        released: [{ name: "notes.files" }],
    });
});

test("bind a declaration to an installation of another space by name, unpinned, block one pinning it, and refuse another kind there", async () => {
    const fixture = await TestSpace.open();
    const stack = await readStack(fixture.database);
    const store = await serve(fixture);

    // submit notes with its tasks dependency bound to an installation of another space
    const acme = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-00000000003a");
    const board = "board";
    const applied = async (target: JsonValue, version?: number) => {
        const tasks = { target, ...(version === undefined ? {} : { version }) };
        const notes = {
            kind: "package" as const,
            packageId: ids.package,
            selection: { kind: "release", version: "2026.9.3" },
            status: "enabled",
            name: "notes",
            bindings: { [ids.package]: { tasks } },
            compute: {},
            capabilities: [],
            directories: [],
            tags: {},
        };
        const submitted = await submit(fixture, store, stack, {
            selection: {
                kind: "checkout" as const,
                machine: ids.machine,
                checkout: ids.checkout,
                directory: "." as const,
            },
            evaluation: {
                export: "personal",
                parameters: {},
                definition: { ...evaluatePersonal(), installations: { notes } },
            },
        });

        return [submitted.conditions["Ready"]?.reason, submitted.conditions["Ready"]?.message];
    };
    const bound = await applied({ type: "installation", name: board, spaceId: acme });
    const rows = await fixture.database
        .select({
            name: binding.table.name,
            target: binding.table.target,
            targetSpaceId: binding.table.targetSpaceId,
        })
        .from(binding.table);

    // keep the binding with the other space and block pinning or rebinding it
    expect({
        bound,
        rows,
        pinned: await applied({ type: "installation", name: board, spaceId: acme }, 2),
        database: StackBindingTarget.safeParse({
            type: "database",
            name: board,
            spaceId: acme,
        }).error?.issues.map((issue) => [issue.code, issue.message]),
    }).toEqual({
        bound: ["Applied", "every declaration applied"],
        rows: [{ name: "tasks", target: board, targetSpaceId: acme }],
        pinned: [
            "Blocked",
            `a binding must select an installation of another space by name without a version, not ${board} in ${acme}`,
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
    const blocked = async (grant: JsonValue) => {
        const submitted = await submit(fixture, store, stack, {
            selection: {
                kind: "checkout" as const,
                machine: ids.machine,
                checkout: ids.checkout,
                directory: "." as const,
            },
            evaluation: {
                export: "personal",
                parameters: {},
                definition: {
                    roles: { reader: { description: "Read", permissions: [] } },
                    relationships: { grant },
                },
            },
        });

        return [submitted.conditions["Ready"]?.reason, submitted.conditions["Ready"]?.message];
    };
    const installationId = "installation-01996ab0-0000-7000-8000-00000000000c";
    const user = "user-01996ab0-0000-7000-8000-00000000000d";
    expect([
        await blocked({ subject: { id: user }, role: { id: foreign } }),
        await blocked({ subject: { id: installationId }, role: "reader" }),
    ]).toEqual([
        ["Blocked", `unknown role: ${foreign}`],
        ["Blocked", `unknown installation: ${installationId}`],
    ]);
});

test("install a package and declare records of its type in one approved plan, applied once the store serves the type", async () => {
    const fixture = await TestSpace.open({ packages: [{ objects: { meter }, tables: [] }] });
    const { database } = fixture;

    // publish notes declaring meters, and require approval of every plan in the space
    const release = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [
            {
                kind: "object",
                kindPackageId: objectPackage.id,
                name: "meter",
                description: describeObject(meter),
            },
        ],
        fixture.environment.crypto,
        { outputs: fixtureOutputs({}) },
    );
    const manifest = await fixture.registry.release(release);
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(space.table)
                .set({ approval: "safe" })
                .where(eq(space.table.id, ids.space)),
    );

    // submit a stack installing notes and declaring a meter
    const notes = {
        kind: "package" as const,
        packageId: ids.package,
        selection: { kind: "release", version: "2026.9.0", manifest },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        tags: {},
    };
    const definition = { installations: { notes }, meters: { main: {} } };
    const stack = await readStack(database);
    const store = await serve(fixture);
    const build = await fixture.recordBuild(
        await fixtureBuild(stack.installation.packageId, "2026.9.0", fixture.environment.crypto),
    );
    await store.mutate(store.as(rollout), {
        id: RequestId.create(fixture.machine.environment),
        calls: [
            rollout.call("submit", {
                ...rollout.target(stack),
                selection: {
                    kind: "checkout",
                    machine: ids.machine,
                    checkout: ids.checkout,
                    directory: ".",
                },
                evaluation: { export: "personal", parameters: {}, definition },
                build,
            }),
        ],
    });

    // plan installing notes and declaring the meter as one plan waiting for approval
    await reconcile(store, "rollout", { scope: ids.space, kind: "stack" });
    const waiting = await readStack(database);
    expect([waiting.conditions["Ready"]?.reason, waiting.plan]).toEqual([
        "AwaitingApproval",
        {
            steps: [
                { action: "create", target: "rollout/notes", risk: "safe", detail: "declare" },
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
    await store.mutate(store.as(rollout), {
        id: RequestId.create(fixture.machine.environment),
        calls: [rollout.call("approve", { ...rollout.target(waiting), plan: digest })],
    });
    await reconcile(store, "rollout", { scope: ids.space, kind: "stack" });
    const installed = await readStack(database);
    expect(installed.conditions["Ready"]?.message).toBe(
        "meters wait for the store to serve their types",
    );

    // apply the meter under the same approval once the reloaded store serves meters
    const reloaded = await serve(fixture, { servers: [meters] });
    await reconcile(reloaded, "rollout", { scope: ids.space, kind: "stack" });
    const applied = await readStack(database);
    const declared = await database.select({ name: meter.table.managerName }).from(meter.table);
    expect([applied.conditions["Ready"]?.reason, applied.appliedRevisionId, declared]).toEqual([
        "Applied",
        applied.revisionId,
        [{ name: "main" }],
    ]);
});

test("install the installable packages a stack's installations depend on, leaving libraries out, and refuse two releases of one package", async () => {
    // publish an installable dependency at two releases, and a library, in the test's world
    const world = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 1) }).environment("test");
    const vault = await fixtureRelease("@acme/vault", "2026.10.0", true, [], world);
    const newer = await fixtureRelease(
        "@acme/vault",
        "2026.10.1",
        true,
        [],
        world,
        vault.package.id,
    );
    const library = await fixtureRelease("@acme/library", "2026.10.0", false, [], world);

    // publish two installable packages depending on them
    const notes = await fixtureRelease("@acme/notes", "2026.10.0", true, [vault, library], world);
    const pages = await fixtureRelease("@acme/pages", "2026.10.0", true, [newer], world);
    const builds = new Map(
        [vault, newer, library, notes, pages].map((each) => [each.manifest, each.build]),
    );
    const read = async (_packageId: PackageId, manifest: string) =>
        present(builds.get(Digest.parse(manifest)), `the build ${manifest}`);

    // install a stack's notes with its installable dependency, and refuse notes next to pages
    const definition = StackDefinition.collections(
        await declaredStack(await installingStack([notes], world), "space", {}, read),
    );
    const refused = await refusal(
        declaredStack(await installingStack([notes, pages], world), "space", {}, read),
    );
    expect([Object.keys(definition.installations).toSorted(), refused]).toEqual([
        ["notes", "vault"],
        ["BAD_REQUEST", "DEFINITION_INVALID", "stack space resolves two releases of @acme/vault"],
    ]);
});

/** A fixture release of a package, with its build. */
interface FixtureRelease {
    /** The registry format. */
    readonly kind: "destack";
    /** The released package. */
    readonly package: Package;
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
    // describe the package as installable, else as a library
    const owner = Package.parse({ id, name, version });
    const declared = isInstallable
        ? [
              {
                  kind: PACKAGE_DECLARATION,
                  kindPackageId: objectPackage.id,
                  name: "default",
                  description: describePackage(PackageHandle.of(owner, { objects: {} })),
              },
          ]
        : [];
    const build = await MemoryBuild.of(owner, declared, environment.crypto, {
        dependencies: resolutions(dependencies),
    });
    const manifest = await Digest.of(
        new TextEncoder().encode(`${name}@${version}`),
        environment.crypto,
    );

    return { kind: "destack" as const, package: owner, manifest, build };
}

/** Build a stack installing some releases, resolving them. */
function installingStack(
    installed: readonly FixtureRelease[],
    environment: Environment,
): Promise<BuildReader> {
    // declare the stack's package and an installation of each release
    const owner = Package.parse({
        id: Identifier.create("package", environment),
        name: "@acme/stack",
        version: "2026.10.0",
    });
    const installations = Object.fromEntries(
        installed.map((each) => [
            each.package.name.slice(each.package.name.lastIndexOf("/") + 1),
            install({ ...each.package, resources: {}, secrets: {} }, {}),
        ]),
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
        { dependencies: resolutions(installed) },
    );
}

/** Record releases as a build's resolved dependencies. */
function resolutions(released: readonly FixtureRelease[]) {
    return Object.fromEntries(
        released.map(({ kind, package: dependency, manifest }) => [
            `${dependency.name}@${dependency.version}`,
            { kind, package: dependency, manifest },
        ]),
    );
}

test("refuse a declaration in the enclosing account from a space that is not the account's home", async () => {
    // apply a role declared in the account from an ordinary space of it
    const fixture = await TestSpace.open();
    const submitted = await fixture.submit(await fixture.serve(), [], {
        roles: { reviewer: { description: "Review notes", permissions: [], scope: "enclosing" } },
    });

    // block the stack at its plan, before writing anything
    expect(submitted).toEqual([
        "Blocked",
        "role reviewer is declared in the enclosing scope, which only its home's stack declares into",
    ]);
});
