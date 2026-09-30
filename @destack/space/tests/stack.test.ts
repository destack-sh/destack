import { spaceTables } from "../src/stack/index.ts";
import { copyScope } from "@destack/access/test";
import { Scope } from "@destack/sync";
import { account } from "@destack/account/object";
import { expect, onTestFinished, test } from "@destack/test";
import { and, type DatabaseConnection, eq, isNotNull } from "@destack/db";
import * as sqlite from "@destack/db/bun";
import { TestDatabase } from "@destack/db/test";
import type { ObjectServer } from "@destack/object/server";
import { ControlLoop } from "@destack/service/control";
import { Plan } from "@destack/resource";
import { identifier } from "@destack/schema";
import type { JsonValue } from "@destack/schema/json";
import * as stackModule from "@template/stack/space";
import type { SpaceDefinition } from "../src/declare/index.ts";
import {
    Approval,
    Binder,
    evaluateStack,
    recordSubmission,
    role,
    resourceBindable,
    serveObjects,
} from "../src/server/index.ts";
import {
    binding,
    type Installation,
    installation,
    type Submission,
    networkPolicy,
    networkPolicyVersion,
    packagePolicy,
    resource,
    space,
} from "../src/object/index.ts";
import { reconcile, spaceOptions } from "./fixture/space.ts";

/** Identifiers of the fixture space, its stack and the checkout it follows. */
const ids = {
    account: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    region: identifier("region").parse("region-01996ab0-0000-7000-8000-000000000004"),
    host: identifier("host").parse("host-01996ab0-0000-7000-8000-000000000005"),
    checkout: identifier("checkout").parse("checkout-01996ab0-0000-7000-8000-000000000006"),
    package: identifier("package").parse("package-01996ab0-0000-7000-8000-000000000007"),
};

/** Create a space's stack installation following the fixture checkout, as its owner does before submitting builds. */
async function createStack(database: DatabaseConnection): Promise<Installation> {
    const now = Date.now();
    const [stack] = await database
        .insert(installation.table)
        .values({
            id: identifier("installation").parse(
                "installation-01996ab0-0000-7000-8000-000000000008",
            ),
            scope: ids.space,
            packageId: ids.package,
            role: "stack",
            selection: { kind: "checkout", host: ids.host, checkout: ids.checkout, directory: "." },
            export: "personal",
            parameters: {},
            alias: "stack",
            createdAt: now,
            updatedAt: now,
        })
        .returning();

    return stack!;
}

/** Read a space's stack installation as it stands. */
async function readStack(database: DatabaseConnection, stack: Installation): Promise<Installation> {
    const [read] = await database
        .select()
        .from(installation.table)
        .where(eq(installation.table.id, stack.id));

    return read!;
}

/** Submit a build to a space's stack as its submit method does, then let the stack controller apply it. */
async function submit(
    server: Pick<ObjectServer, "database" | "invoke" | "controllers">,
    stack: Installation,
    submission: Submission,
): Promise<Installation> {
    // record the submission as the system
    const current = await readStack(server.database, stack);
    await server.database.transaction((transaction) =>
        recordSubmission(
            transaction,
            (object, name, input) =>
                server.invoke(transaction, stack.scope, object, name, input, Date.now()),
            current,
            submission,
        ),
    );

    // apply it
    await reconcile(server, "installation", { scope: ids.space, role: "stack" });

    return readStack(server.database, stack);
}

/** The checkout build a revision evaluated. */
const build = {
    kind: "checkout" as const,
    host: ids.host,
    checkout: ids.checkout,
    directory: "." as const,
    manifest: "a".repeat(64),
};

test("apply a stack submission, reapply it unchanged, and retire removed records once approved", async () => {
    const database = await sqlite.connect(":memory:", spaceTables);
    onTestFinished(() => database.close());
    await database.migrate(spaceTables);

    // register a space without a stack
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
    const definition = evaluateStack(stackModule, "personal", {});
    const stack = await createStack(database);
    const submission = {
        selection: {
            kind: "checkout" as const,
            host: ids.host,
            checkout: ids.checkout,
            directory: "." as const,
        },
        build,
        evaluation: { export: "personal", parameters: {}, definition },
    };

    // apply the submission, creating every declared record once
    const server = serveObjects(await spaceOptions(database));
    const first = await submit(server, stack, submission);
    expect([first.appliedRevisionId, first.conditions.ready?.reason]).toEqual([
        first.revisionId,
        "Applied",
    ]);
    const resources = await database.select().from(resource.table);
    expect(resources.map((row) => [row.name, row.kind, row.managerName])).toEqual([
        ["main", "database", "main"],
        ["files", "bucket", "files"],
        ["credentials", "vault", "credentials"],
    ]);
    const inForce = await database
        .select({
            name: networkPolicy.table.managerName,
            level: networkPolicy.table.level,
            generation: networkPolicy.table.generation,
        })
        .from(networkPolicy.table)
        .innerJoin(
            networkPolicyVersion.table,
            eq(networkPolicyVersion.table.id, networkPolicy.table.currentVersionId),
        );
    const packages = await database
        .select({
            name: packagePolicy.table.managerName,
            level: packagePolicy.table.level,
            generation: packagePolicy.table.generation,
        })
        .from(packagePolicy.table);
    expect([inForce, packages]).toEqual([
        [{ name: "network", level: "space", generation: 1 }],
        [{ name: "packages", level: "space", generation: 1 }],
    ]);

    // submit the same build again without changes, onto the same revision
    const again = await submit(server, stack, submission);
    expect(again.appliedRevisionId).toBe(first.appliedRevisionId);

    // remove a resource from the definition and request its deletion
    const { files: _files, ...remaining } = definition.resources as Record<
        string,
        SpaceDefinition[string]
    >;
    const removal = {
        ...submission,
        evaluation: {
            ...submission.evaluation,
            definition: { ...definition, resources: remaining },
        },
    };
    const waiting = await submit(server, stack, removal);
    expect([waiting.conditions.ready?.reason, waiting.plan]).toEqual([
        "AwaitingApproval",
        {
            steps: [
                {
                    action: "delete",
                    target: "resource/files",
                    risk: "destructive",
                    detail: "retire",
                },
            ],
        },
    ]);

    // approve the destructive plan, then retire the resource
    const digest = await Plan.digest(waiting.plan!);
    await database.transaction(async (transaction) =>
        (await Approval.of(transaction, waiting)).grant(digest, (object, name, input) =>
            server.invoke(transaction, ids.space, object, name, input, Date.now()),
        ),
    );
    await reconcile(server, "installation", { scope: ids.space, role: "stack" });
    const [retired] = await database
        .select()
        .from(resource.table)
        .where(eq(resource.table.name, "files"));
    expect(retired!.deletionRequestedAt).not.toBeNull();
});

test("apply a waiting stack again once its resources turn ready, without waking itself while it waits", async () => {
    const test = await TestDatabase.create("sqlite", spaceTables, { isMigrated: true });
    onTestFinished(() => test.close());
    const database = test.database;

    // administer a space on this host
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));

    // install an application only once the stack's main database is ready
    const plain = serveObjects(await spaceOptions(database));
    const served = plain.objects.find((type) => type.same(installation))!;
    const waiting = installation.declare({
        after: [resource],
        resolve: async (_name, declared, context) => {
            // wait while the main database is not applied
            const [main] = await context.database
                .select({ conditions: resource.table.conditions })
                .from(resource.table)
                .where(
                    and(
                        eq(resource.table.scope, identifier("space").parse(context.scope)),
                        eq(resource.table.name, "main"),
                    ),
                );
            if (main?.conditions.ready?.reason !== "Applied") {
                context.wait("main is not ready");
            }

            return declared;
        },
        values: served.declaration!.values as never,
    });

    // apply a stack with the application waiting for the database
    const notes = {
        package: { id: ids.package, name: "@example/notes", version: "2026.9.0" },
        status: "enabled",
        alias: "notes",
        bindings: {},
        compute: {},
        tags: {},
    };
    const stack = await createStack(database);
    const submission = {
        selection: {
            kind: "checkout" as const,
            host: ids.host,
            checkout: ids.checkout,
            directory: "." as const,
        },
        build,
        evaluation: {
            export: "personal",
            parameters: {},
            definition: { ...evaluateStack(stackModule, "personal", {}), installations: { notes } },
        },
    };
    const server = serveObjects(await spaceOptions(database, { declared: [waiting] }));
    const applied = await submit(server, stack, submission);
    expect(applied.conditions.ready?.reason).toBe("WaitingForResources");

    // apply it again while it waits without writing, since each write would wake the controller again
    const position = await database.log.position();
    await reconcile(server, "installation", { scope: ids.space, role: "stack" });
    expect(await database.log.position()).toEqual(position);

    // report the database ready, and let the controller apply the waiting stack
    const stopping = new AbortController();
    const stacks = server.controllers().filter((each) => each.name === "installation");
    const running = new ControlLoop(database, stacks, {
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await database
        .update(resource.table)
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
        .where(and(eq(resource.table.scope, ids.space), eq(resource.table.name, "main")));
    await expect
        .poll(async () => (await readStack(database, stack)).conditions.ready?.reason, {
            timeout: 5000,
        })
        .toBe("Applied");
    const installed = await database
        .select({ alias: installation.table.alias, role: installation.table.role })
        .from(installation.table)
        .where(eq(installation.table.scope, ids.space));
    expect(installed.sort((left, right) => left.alias.localeCompare(right.alias))).toEqual([
        { alias: "notes", role: "application" },
        { alias: "stack", role: "stack" },
    ]);
});

test("replace an installation's own binding once its stack binds the declaration, releasing the resource it owned", async () => {
    const test = await TestDatabase.create("sqlite", spaceTables, { isMigrated: true });
    onTestFinished(() => test.close());
    const database = test.database;

    // administer a space with a stack that installs notes without its main database
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
    const notes = {
        package: { id: ids.package, name: "@example/notes", version: "2026.9.0" },
        status: "enabled",
        alias: "notes",
        bindings: {},
        compute: {},
        tags: {},
    };
    const stack = await createStack(database);
    const submission = {
        selection: {
            kind: "checkout" as const,
            host: ids.host,
            checkout: ids.checkout,
            directory: "." as const,
        },
        build,
        evaluation: {
            export: "personal",
            parameters: {},
            definition: { ...evaluateStack(stackModule, "personal", {}), installations: { notes } },
        },
    };
    const server = serveObjects(await spaceOptions(database));
    await submit(server, stack, submission);

    // attach a database notes owns to its main declaration, as its deployment does
    const [installed] = await database
        .select()
        .from(installation.table)
        .where(eq(installation.table.alias, "notes"));
    const need = {
        package: notes.package,
        name: "main",
        kind: "database",
        spec: { tier: "zonal" },
        state: {},
    };
    await database.transaction((transaction) =>
        new Binder([resourceBindable]).attach(
            {
                database: transaction,
                invoke: (object, name, input) =>
                    server.invoke(transaction, ids.space, object, name, input, Date.now()),
            },
            installed!,
            [need],
        ),
    );

    // bind the declaration in the stack to replace the owned binding and release its resource
    const bindings = {
        [ids.package]: { main: { target: { type: "resource", name: "main" }, state: {} } },
    };
    await submit(server, stack, {
        ...submission,
        evaluation: {
            ...submission.evaluation,
            definition: {
                ...submission.evaluation.definition,
                installations: { notes: { ...notes, bindings } },
            },
        },
    });
    const [main] = await database
        .select()
        .from(resource.table)
        .where(eq(resource.table.name, "main"));
    const released = await database
        .select({ name: resource.table.name })
        .from(resource.table)
        .where(isNotNull(resource.table.deletionRequestedAt));
    const bound = await database
        .select({
            name: binding.table.name,
            target: binding.table.target,
            manager: binding.table.managerInstallationId,
        })
        .from(binding.table);
    expect({ bound, released }).toEqual({
        bound: [{ name: "main", target: main!.id, manager: stack.id }],
        released: [{ name: "notes.main" }],
    });
});

test("block a stack relating a role or installation of another space by identifier", async () => {
    const database = await sqlite.connect(":memory:", spaceTables);
    onTestFinished(() => database.close());
    await database.migrate(spaceTables);

    // register the space and a role in another space of the account
    const now = Date.now();
    const other = identifier("space").parse("space-01996ab0-0000-7000-8000-00000000000a");
    await database.insert(space.table).values([
        { id: ids.space, scope: ids.account, name: "personal", createdAt: now, updatedAt: now },
        { id: other, scope: ids.account, name: "shared", createdAt: now, updatedAt: now },
    ]);
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
    const foreign = identifier("role").parse("role-01996ab0-0000-7000-8000-00000000000b");
    await database.insert(role.table).values({
        id: foreign,
        scope: other,
        name: "editor",
        description: "Edit",
        createdAt: now,
        updatedAt: now,
    });
    const stack = await createStack(database);
    const server = serveObjects(await spaceOptions(database));

    // submit a stack binding the foreign role, then one relating a foreign installation
    const blocked = async (grant: JsonValue) => {
        const submitted = await submit(server, stack, {
            selection: {
                kind: "checkout" as const,
                host: ids.host,
                checkout: ids.checkout,
                directory: "." as const,
            },
            build: { ...build, manifest: String(Object.keys(grant as object).length).repeat(64) },
            evaluation: {
                export: "personal",
                parameters: {},
                definition: {
                    roles: { reader: { description: "Read", permissions: [] } },
                    relationships: { grant },
                },
            },
        });

        return [submitted.conditions.ready?.reason, submitted.conditions.ready?.message];
    };
    const installationId = "installation-01996ab0-0000-7000-8000-00000000000c";
    const user = "user-01996ab0-0000-7000-8000-00000000000d";
    expect([
        await blocked({ subject: { user }, role: { id: foreign } }),
        await blocked({ subject: { installation: { id: installationId } }, role: "reader" }),
    ]).toEqual([
        ["Blocked", `the space has no role ${foreign}`],
        ["Blocked", `the space has no installation ${installationId}`],
    ]);
});
