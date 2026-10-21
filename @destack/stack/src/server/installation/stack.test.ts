import { defineObject, role } from "@destack/object";
import { STACKS } from "@destack/account/test";
import { space } from "@destack/account/object";
import { type SpaceOptions, serveSpace } from "../index.ts";
import { pathToFileURL } from "node:url";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { mkdtemp, rm } from "node:fs/promises";
import { spaceReplicated } from "../../stack/index.ts";
import { PACKAGE_DECLARATION, PACKAGE_KIND_PACKAGE_ID, PackageHandle } from "@destack/runtime";
import { expect, onTestFinished, test } from "@destack/test";
import {
    and,
    type DatabaseConnection,
    databaseProvider,
    defineDatabase,
    eq,
    isNotNull,
} from "@destack/db";
import * as sqlite from "@destack/db/bun";
import { defineServer, ObjectStore } from "@destack/object/server";
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
import { describeObject } from "@destack/object/inspect";
import { Package, PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { defineBucket } from "@destack/bucket";
import { defineStack, install, StackBindingTarget, StackDefinition } from "../../declare/index.ts";
import stackPackage from "../../package.ts";
import { declaredStack } from "./stack.ts";
import * as object from "../../object/index.ts";
import { binding, type Installation, installation, type Submission } from "../../object/index.ts";
import {
    evaluateStack,
    fixtureBuild,
    fixtureOutputs,
    reconcile,
    settle,
    spaceDatabase,
    StackFixture,
    ids,
} from "../../test/index.ts";
import { principal } from "@destack/access";
import { ObjectReference } from "@destack/sync";
import { RequestId } from "@destack/runtime/request";
import { VirtualClock } from "@destack/environment/simulation";

/** A database a stack declares for its own use, beside the space's main database. */
const archiveDatabase = defineDatabase({ name: "archive", tables: [] });

/** A bucket a stack declares for its installations' files. */
const filesBucket = defineBucket({ name: "files", spec: {} });

/** A stack keeping an archive database and a files bucket. */
const personal = defineStack({
    resources: {
        archive: { declaration: archiveDatabase, retention: { within: { days: 30 } }, tags: {} },
        files: { declaration: filesBucket, retention: { within: { days: 30 } }, tags: {} },
    },
});

/** Create a space's stack installation following the fixture checkout. */
async function createStack(database: DatabaseConnection): Promise<Installation> {
    const now = Date.now();
    const [stack] = await database
        .insert(installation.table)
        .values({
            id: schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-000000000008"),
            scope: ids.space,
            packageId: ids.stackPackage,
            role: "stack",
            selection: {
                kind: "checkout",
                machine: ids.machine,
                checkout: ids.checkout,
                directory: ".",
            },
            export: "personal",
            parameters: {},
            name: "stack",
            createdAt: now,
            updatedAt: now,
        })
        .returning();

    return present(stack, "the created stack");
}

/** Read a space's stack installation as it stands. */
async function readStack(database: DatabaseConnection, stack: Installation): Promise<Installation> {
    const [read] = await database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, stack.id));

    return present(read, `stack ${stack.id}`);
}

/** Submit a build of a manifest to a space's stack and let the stack controller apply it. */
async function submit(
    fixture: StackFixture,
    store: Pick<ObjectStore, "database" | "serving" | "mutate" | "controllers">,
    stack: Installation,
    submission: Omit<Submission, "build">,
): Promise<Installation> {
    // record the stack's build and the submission pinned as its evaluation is, as the system
    const build = await fixture.recordBuild(await fixtureBuild(stack.packageId, "2026.9.0"));
    const evaluation =
        submission.evaluation === undefined
            ? {}
            : {
                  evaluation: {
                      ...submission.evaluation,
                      definition: await fixture.pin(submission.evaluation.definition),
                  },
              };
    await store.mutate(store.serving(installation), {
        id: RequestId.create(Date.now()),
        calls: [
            installation.call("submit", {
                ...installation.target(stack),
                ...submission,
                ...evaluation,
                build,
            }),
        ],
    });

    // apply it
    await reconcile(store, "installation", { scope: ids.space, role: "stack" });

    return readStack(store.database, stack);
}

/** Evaluate the personal stack. */
function evaluatePersonal(): JsonObject {
    return evaluateStack({ personal }, "personal", {});
}

/** Serve a space's objects, keeping its databases as SQLite files in a scratch directory. */
async function serve(fixture: StackFixture, options: Partial<SpaceOptions> = {}) {
    const directory = await mkdtemp(join(tmpdir(), "destack-stack-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const databases = databaseProvider(
        new sqlite.BunSqliteDatabaseHost(pathToFileURL(`${directory}/`)),
        object.database,
    );

    return serveSpace(await fixture.options({ providers: [databases], ...options })).store;
}

/** A type another package keeps, declared by stacks and waiting for their main database. */
const gauge = defineObject({
    name: "gauge",
    plural: "gauges",
    scope: space,
    declarable: { schema: schema.object({}) },
    fields: {},
}).implement({
    declaration: {
        after: [object.database],
        resolve: async (_name, declared, context) => {
            // wait while the main database is not applied
            const [main] = await context.transaction.read(object.database.table).findMany({
                where: {
                    scope: schema.identifier("space").parse(context.scope),
                    name: "main",
                },
            });
            if (main?.conditions["ready"]?.reason !== "Applied") {
                context.wait("main is not ready");
            }

            return declared;
        },
        values: () => ({}),
    },
});

/** A space's database keeping gauges beside the space's tables. */
const gaugeDatabase = defineDatabase({
    name: "space",
    tables: [
        ...spaceDatabase.tables.filter((table) => !spaceDatabase.replicates(table)),
        ...gauge.tables,
    ],
    replicated: spaceReplicated,
});

/** The server of gauges in the space. */
const gauges = defineServer(PackageHandle.of(gauge.package, { objects: { gauge } }), {
    objects: [gauge],
});

/** The package defining the object declarations builds list. */
const objectPackage = PackageId.parse("package-01a0d462-1b26-7401-be27-6258e0a32141");

/** A type an installed package declares, which stacks declare once the store serves it. */
const meter = defineObject({
    name: "meter",
    plural: "meters",
    scope: space,
    declarable: { schema: schema.object({}) },
    fields: {},
}).implement({ declaration: { values: () => ({}) } });

/** A space's database keeping meters beside the space's tables. */
const meterDatabase = defineDatabase({
    name: "space",
    tables: [
        ...spaceDatabase.tables.filter((table) => !spaceDatabase.replicates(table)),
        ...meter.tables,
    ],
    replicated: spaceReplicated,
});

/** The server of meters in the space. */
const meters = defineServer(PackageHandle.of(meter.package, { objects: { meter } }), {
    objects: [meter],
});

test("apply a stack submission, reapply it unchanged, and retire removed records once approved", async () => {
    const fixture = await StackFixture.open();
    const { database } = fixture;

    // register a space without a stack
    const definition = evaluatePersonal();
    const stack = await createStack(database);
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
    const now = Date.now();
    const store = await serve(fixture, { clock: new VirtualClock(now) });
    const first = await submit(fixture, store, stack, submission);
    expect([first.appliedRevisionId, first.conditions["ready"]?.reason]).toEqual([
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
    const { archive: _archive, ...remaining } = schema.jsonObject().parse(definition["resources"]);
    const removal = {
        ...submission,
        evaluation: {
            ...submission.evaluation,
            definition: { ...definition, resources: remaining },
        },
    };
    const waiting = await submit(fixture, store, stack, removal);
    expect([waiting.conditions["ready"]?.reason, waiting.plan]).toEqual([
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
    const digest = await Plan.digest(present(waiting.plan, "the waiting plan"));
    await store.mutate(store.serving(installation), {
        id: RequestId.create(Date.now()),
        calls: [installation.call("approve", { ...installation.target(waiting), plan: digest })],
    });
    await reconcile(store, "installation", { scope: ids.space, role: "stack" });
    const [retired] = await database
        .select()
        .from(object.database.table)
        .where(eq(object.database.table.name, "archive"));
    expect(present(retired, "the retired database").deletedAt).toBe(now);
});

test("apply a waiting stack again once its resources turn ready, without waking itself while it waits", async () => {
    const fixture = await StackFixture.open({ database: gaugeDatabase });
    const { database } = fixture;

    // administer a space on this machine
    const now = Date.now();

    // apply a stack with a gauge waiting for the database
    const notes = {
        package: { id: ids.package, name: "@example/notes", version: "2026.9.0" },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        tags: {},
    };
    const stack = await createStack(database);
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
    const store = await serve(fixture, { servers: [gauges] });
    const applied = await submit(fixture, store, stack, submission);
    expect(applied.conditions["ready"]?.reason).toBe("WaitingForResources");

    // apply it again while it waits without writing
    const position = await database.log.position();
    await reconcile(store, "installation", { scope: ids.space, role: "stack" });
    expect(await database.log.position()).toEqual(position);

    // apply the waiting stack once its database turns ready
    const ready = await database.log.position();
    await database
        .update(object.database.table)
        .set({
            conditions: {
                ready: {
                    status: "true",
                    observedGeneration: 1,
                    reason: "Applied",
                    message: "0 steps applied",
                    lastTransitionAt: now,
                },
            },
        })
        .where(
            and(eq(object.database.table.scope, ids.space), eq(object.database.table.name, "main")),
        );
    await settle(
        database,
        store.controllers().filter((each) => each.name === "installation"),
        ready,
    );
    const installed = await database
        .select({ name: installation.table.name, role: installation.table.role })
        .from(installation.table)
        .where(eq(installation.table.scope, ids.space));
    const managed = await database.select({ name: gauge.table.managerName }).from(gauge.table);
    expect([
        (await readStack(database, stack)).conditions["ready"]?.reason,
        installed.toSorted((left, right) => left.name.localeCompare(right.name)),
        managed,
    ]).toEqual([
        "Applied",
        [
            { name: "notes", role: "application" },
            { name: "stack", role: "stack" },
        ],
        [{ name: "main" }],
    ]);
});

test.each(["bucket", "resource"])(
    "replace an installation's own binding once its stack binds the declaration as a %s, releasing the resource it owned",
    async (type) => {
        const fixture = await StackFixture.open();
        const { database } = fixture;

        // administer a space with a stack that installs notes without binding its files
        const notes = {
            package: { id: ids.package, name: "@example/notes", version: "2026.9.0" },
            status: "enabled",
            name: "notes",
            bindings: {},
            compute: {},
            capabilities: [],
            directories: [],
            tags: {},
        };
        const stack = await createStack(database);
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
                definition: { ...evaluatePersonal(), installations: { notes } },
            },
        };
        const store = await serve(fixture);
        await submit(fixture, store, stack, submission);

        // bind a bucket notes owns to its files declaration, as its deploy does
        const [installed] = await database
            .select()
            .from(installation.table)
            .where(eq(installation.table.name, "notes"));
        const owner = principal.installation.reference(ids.space, present(installed, "notes").id);
        const ownedId = object.bucket.generateId();
        await store.mutate(
            store.serving(object.bucket),
            {
                id: RequestId.create(Date.now()),
                calls: [
                    object.bucket.call("create", {
                        id: ownedId,
                        name: "notes.files",
                        definitionPackageId: ids.package,
                        definitionVersion: "2026.9.0",
                        definitionName: "files",
                        spec: {},
                        ...ObjectReference.columns("owner", owner),
                    }),
                    binding.call("create", {
                        installationId: present(installed, "notes").id,
                        packageId: ids.package,
                        name: "files",
                        target: ownedId,
                        state: {},
                    }),
                ],
            },
            { scope: ids.space },
        );

        // bind the declaration in the stack to replace the owned binding and release its bucket
        const bindings = {
            [ids.package]: { files: { target: { type, name: "files" }, state: {} } },
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
        expect({ bound, released }).toEqual({
            bound: [
                { name: "files", target: present(files, "the files bucket").id, manager: stack.id },
            ],
            released: [{ name: "notes.files" }],
        });
    },
);

test("bind a declaration to an installation of another space by name, unpinned, block one pinning it, and refuse another kind there", async () => {
    const fixture = await StackFixture.open();
    const stack = await createStack(fixture.database);
    const store = await serve(fixture);

    // submit notes with its tasks dependency bound to an installation of another space
    const acme = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-00000000003a");
    const board = "board";
    const applied = async (target: JsonValue, version?: number) => {
        const tasks = { target, ...(version === undefined ? {} : { version }), state: {} };
        const notes = {
            package: { id: ids.package, name: "@example/notes", version: "2026.9.0" },
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

        return [submitted.conditions["ready"]?.reason, submitted.conditions["ready"]?.message];
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
            `a binding selects another space's installation by name, unpinned, not ${board} in ${acme}`,
        ],
        database: [["unrecognized_keys", 'Unrecognized key: "spaceId"']],
    });
});

test("block a stack relating a role or installation of another space by identifier", async () => {
    const fixture = await StackFixture.open();
    const { database } = fixture;

    // register the space and a role in another space of the account
    const now = Date.now();
    const other = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-00000000000a");
    await database.insert(space.table).values({
        id: other,
        scope: ids.account,
        name: "shared",
        stack: STACKS.space,
        createdAt: now,
        updatedAt: now,
    });
    const foreign = schema.identifier("role").parse("role-01996ab0-0000-7000-8000-00000000000b");
    await database.insert(role.table).values({
        id: foreign,
        scope: other,
        name: "editor",
        description: "Edit",
        createdAt: now,
        updatedAt: now,
    });
    const stack = await createStack(database);
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

        return [submitted.conditions["ready"]?.reason, submitted.conditions["ready"]?.message];
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
    const fixture = await StackFixture.open({ database: meterDatabase });
    const { database } = fixture;

    // publish notes declaring meters, and require approval of every plan in the space
    const release = await MemoryBuild.of(
        Package.parse({ id: ids.package, name: "@example/notes", version: "2026.9.0" }),
        [
            {
                kind: "object",
                kindPackageId: objectPackage,
                name: "meter",
                description: describeObject(meter),
            },
        ],
        { outputs: fixtureOutputs({}) },
    );
    const manifest = await fixture.registry.push(release);
    await database
        .update(space.table)
        .set({ approval: "safe" })
        .where(eq(space.table.id, ids.space));

    // submit a stack installing notes and declaring a meter
    const notes = {
        package: { id: ids.package, name: "@example/notes", version: "2026.9.0", manifest },
        status: "enabled",
        name: "notes",
        bindings: {},
        compute: {},
        capabilities: [],
        directories: [],
        tags: {},
    };
    const definition = { installations: { notes }, meters: { main: {} } };
    const stack = await createStack(database);
    const store = await serve(fixture);
    const build = await fixture.recordBuild(await fixtureBuild(stack.packageId, "2026.9.0"));
    await store.mutate(store.serving(installation), {
        id: RequestId.create(Date.now()),
        calls: [
            installation.call("submit", {
                ...installation.target(stack),
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
    await reconcile(store, "installation", { scope: ids.space, role: "stack" });
    const waiting = await readStack(database, stack);
    expect([waiting.conditions["ready"]?.reason, waiting.plan]).toEqual([
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
    const digest = await Plan.digest(present(waiting.plan, "the waiting plan"));
    await store.mutate(store.serving(installation), {
        id: RequestId.create(Date.now()),
        calls: [installation.call("approve", { ...installation.target(waiting), plan: digest })],
    });
    await reconcile(store, "installation", { scope: ids.space, role: "stack" });
    const installed = await readStack(database, stack);
    expect(installed.conditions["ready"]?.message).toBe(
        "meters wait for the store to serve their types",
    );

    // apply the meter under the same approval once the reloaded store serves meters
    const reloaded = await serve(fixture, { servers: [meters] });
    await reconcile(reloaded, "installation", { scope: ids.space, role: "stack" });
    const applied = await readStack(database, stack);
    const declared = await database.select({ name: meter.table.managerName }).from(meter.table);
    expect([applied.conditions["ready"]?.reason, applied.appliedRevisionId, declared]).toEqual([
        "Applied",
        applied.revisionId,
        [{ name: "main" }],
    ]);
});

test("install the installable packages a stack's installations depend on, leaving libraries out, and refuse two releases of one package", async () => {
    // publish an installable dependency at two releases, and a library
    const vault = await fixtureRelease("@acme/vault", "2026.10.0", true, []);
    const newer = await fixtureRelease("@acme/vault", "2026.10.1", true, [], vault.package.id);
    const library = await fixtureRelease("@acme/library", "2026.10.0", false, []);

    // publish two installable packages depending on them
    const notes = await fixtureRelease("@acme/notes", "2026.10.0", true, [vault, library]);
    const pages = await fixtureRelease("@acme/pages", "2026.10.0", true, [newer]);
    const builds = new Map(
        [vault, newer, library, notes, pages].map((each) => [each.manifest, each.build]),
    );
    const read = async (_packageId: PackageId, manifest: string) =>
        present(builds.get(Digest.parse(manifest)), `the build ${manifest}`);

    // install a stack's notes with its installable dependency, and refuse notes beside pages
    const definition = StackDefinition.collections(
        await declaredStack(await installingStack([notes]), "space", {}, read),
    );
    const refused = await declaredStack(await installingStack([notes, pages]), "space", {}, read)
        .then(() => undefined)
        .catch((error: unknown) => (error instanceof Error ? error.message : error));
    expect([Object.keys(definition.installations).toSorted(), refused]).toEqual([
        ["notes", "vault"],
        "stack space resolves two releases of @acme/vault",
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
    id: string = Identifier.create("package"),
): Promise<FixtureRelease> {
    const owner = Package.parse({ id, name, version });
    const declared = isInstallable
        ? [
              {
                  kind: PACKAGE_DECLARATION,
                  kindPackageId: PACKAGE_KIND_PACKAGE_ID,
                  name: "default",
                  description: {
                      api: { name: owner.name, procedures: [] },
                      objects: {},
                      adapters: {},
                      views: [],
                      permissions: {},
                      capabilities: {},
                  },
              },
          ]
        : [];
    const build = await MemoryBuild.of(owner, declared, {
        dependencies: resolutions(dependencies),
    });
    const manifest = await Digest.of(new TextEncoder().encode(`${name}@${version}`));

    return { kind: "destack" as const, package: owner, manifest, build };
}

/** Build a stack installing some releases, resolving them. */
function installingStack(installed: readonly FixtureRelease[]): Promise<BuildReader> {
    const owner = Package.parse({
        id: Identifier.create("package"),
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
