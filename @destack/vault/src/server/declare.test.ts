import { MemoryBuild } from "@destack/package/test";
import { testCallKey } from "@destack/service/test";
import { VaultKind } from "../declare/kind.ts";
import { accessRelationship, principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { AccessFixture } from "@destack/access/test";
import { account } from "@destack/account/object";
import { expect, onTestFinished, single, test } from "@destack/test";
import { type DatabaseConnection, defineDatabase, type Dialect, eq, type Table } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { DirectoryStore, directoryTables, ZONE_SCOPE, zoneTable } from "@destack/directory";
import { SystemCall } from "@destack/object";
import { ObjectServer, Subscriber } from "@destack/object/server";
import { ResourceContext } from "@destack/resource/context";
import { present, schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { type Controller, ControlLoop } from "@destack/service/control";
import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { defineSpace } from "@destack/space";
import { deployment, installation, space, type Installation } from "@destack/space/object";
import {
    applyStack,
    Bindable,
    type BinderCall,
    Binder,
    ConsumerController,
    type OpenBuild,
    recordSubmission,
    serveInstallations,
    servePolicies,
    serveRevisions,
    serveRoles,
} from "@destack/space/server";
import { spaceObjects } from "@destack/space/service";
import { spaceCopies, spaceTables } from "@destack/space/stack";
import { v7 } from "uuid";
import { defineVault } from "../declare/index.ts";
import { LocalKeyring } from "@destack/identity";
import { SecretVersion, secretVersion, vault } from "../object/index.ts";
import { SecretClient } from "../object/index.ts";
import { KeyringVaultHost } from "../key/index.ts";
import { vaultService } from "../service/index.ts";
import { vaultDatabase } from "../stack/index.ts";
import { implementVault, secret, secretBindable } from "./index.ts";

/** The vault declared by the fixture stack. */
export const credentials = defineVault({ name: "credentials", spec: {} });

/** Identifiers of the fixture space and its stack. */
const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    stack: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000003"),
    region: schema.identifier("region").parse("region-01996ab0-0000-7000-8000-000000000004"),
    package: schema.identifier("package").parse("package-01996ab0-0000-7000-8000-000000000005"),
    notes: schema.identifier("package").parse("package-01996ab0-0000-7000-8000-000000000006"),
};

/** The longest a call sent between the space and the vault service and its copy back take here. */
const COPY_TIMEOUT_MILLISECONDS = 10_000;

/** A cell's space database copying the vaults, secrets and versions the vault service keeps. */
const cellDatabase = defineDatabase({
    name: "space",
    tables: spaceTables,
    copies: [...spaceCopies, vault.table, secret.table, secretVersion.table],
});

/** The notes installation with a mail token secret. */
const notes = {
    package: { id: ids.notes, name: "@example/notes", version: "2026.9.0" },
    status: "enabled" as const,
    alias: "notes",
    compute: {},
    tags: {},
};

/** A stack declaring a vault, a secret, and an application reading it, fixed at a version or not. */
function stack(pin?: number, isSecretDeclared = true) {
    const selection = {
        target: { type: "secret", name: "mail" },
        ...(pin === undefined ? {} : { version: pin }),
        state: {},
    };

    return defineSpace({
        resources: { credentials: { declaration: credentials, retention: "forever", tags: {} } },
        ...(isSecretDeclared
            ? { secrets: { mail: { vault: "credentials", name: "mail-token" } } }
            : {}),
        installations: {
            notes: {
                ...notes,
                bindings: isSecretDeclared ? { [ids.notes]: { token: selection } } : {},
            },
        },
    }).definition;
}

/** The release build the revisions evaluated. */
const build = { kind: "release" as const, version: "2026.9.0", manifest: "b".repeat(64) };

/** Open every build as an empty build of its package. */
const openBuild: OpenBuild = async (packageId) =>
    (
        await MemoryBuild.write(new Map(), {
            package: {
                id: packageId,
                name: "@example/fixture",
                version: "2026.9.0",
            },
        })
    ).reader;

/** Register the space placed in the region, its scopes and its stack installation, as the space service does. */
async function register(database: DatabaseConnection): Promise<void> {
    const now = Date.now();
    await database.insert(space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });
    const copies = new AccessFixture(database);
    await copies.copyScope(account.reference(Scope.universe.id, ids.account));
    await copies.copyScope(space.reference(ids.account, ids.space));
    await database.insert(zoneTable).values({
        id: ids.space,
        scope: ZONE_SCOPE,
        parent: ids.account,
        cell: ids.region,
        epoch: 1,
    });
    await database.insert(installation.table).values({
        id: ids.stack,
        scope: ids.space,
        packageId: ids.package,
        role: "stack",
        selection: { kind: "release", version: "2026.9.0" },
        export: "personal",
        parameters: {},
        alias: "stack",
        createdAt: now,
        updatedAt: now,
    });
}

/** Run some controllers over a database until the test ends, failing it on a reported failure. */
function run(database: DatabaseConnection, controllers: readonly Controller[]): void {
    const stopping = new AbortController();
    const running = new ControlLoop(database, controllers, {
        report: (_controller, _key, error) => {
            throw error;
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
}

/** Serve a cell's space objects copying the vault service's objects, and the vault service following the space's rows, each sending and copying in the background. */
async function serveCell(dialect: Dialect) {
    // keep the space's rows, vaults' rows, contents and claims apart
    const opened = await TestDatabase.create(dialect, cellDatabase, { isMigrated: true });
    onTestFinished(() => opened.close());
    const kept = await TestDatabase.create(dialect, vaultDatabase, { isMigrated: true });
    onTestFinished(() => kept.close());
    const universe = await TestDatabase.create("sqlite", directoryTables, { isMigrated: true });
    onTestFinished(() => universe.close());
    const { database } = opened;
    const directory = new DirectoryStore(universe.database);
    await register(database);

    // serve the vaults following the space's rows from the space's objects
    const keyring = await LocalKeyring.import(
        1,
        new Map([[1, crypto.getRandomValues(new Uint8Array(32))]]),
    );
    const vaults = new KeyringVaultHost(kept.database, keyring, "eu");
    let spaces: ObjectServer | undefined;
    const vaulted = implementVault({
        database: kept.database,
        callKey: testCallKey,
        directory,
        keyring,
        location: "eu",
        machine: null,
        cell: ids.region,
        spaces: {
            stream: (subscription, signal) =>
                present(spaces, "the space's objects").uplink.stream(subscription, signal),
            receive: (mutation) => present(spaces, "the space's objects").uplink.receive(mutation),
        },
        recovery: { days: 1 },
    });

    // serve the space's objects with copies of the vaults, secrets and versions
    const source = vaulted.objects;
    const binder = new Binder([Bindable.resource(VaultKind, vault)], [secretBindable]);
    const server: ObjectServer = new ObjectServer({
        objects: {
            ...Object.fromEntries(
                Object.entries(spaceObjects).filter(
                    ([name]) => name !== "space" && name !== "connection",
                ),
            ),
            installationRevision: serveRevisions(openBuild),
        },
        policies: [space, ...source.objects],
        sources: [source],
        subscriber: Subscriber.of(source.uplink, async () =>
            server.source.sourceSubscriptions(ids.region).map((entry) => entry.subscription),
        ),
        directory,
        database,
        callKey: testCallKey,
        origin: { package: spaceObjects.space.package, service: "space" },
    });
    spaces = server;

    // send, copy, specify and provision in the background
    run(database, [
        ...server.controllers().filter((each) => ["sends", "replica"].includes(each.name)),
        new ConsumerController(server, [vault], { regionId: ids.region }),
    ]);
    run(kept.database, present(vaulted.controllers, "the vault service's controllers"));

    return { database, kept: kept.database, vaults, server, vaulted, binder, directory };
}

/** Build the system calls a binder writes with in a transaction of the space's objects. */
function binderCall(server: ObjectServer, transaction: DatabaseConnection): BinderCall {
    const context = { database: transaction, scope: ids.space, now: Date.now() };

    return {
        database: transaction,
        invoke: server.invoker(context),
        change: (object, name, input) => server.change(context, object, name, input),
    };
}

/** Read the rows of a table until one matches, as copies arrive. */
async function copied<Row>(
    read: () => PromiseLike<readonly Row[]>,
    isWanted: (row: Row) => boolean,
) {
    let found: Row | undefined;
    await expect
        .poll(
            async () => {
                found = (await read()).find(isWanted);

                return found !== undefined;
            },
            { timeout: COPY_TIMEOUT_MILLISECONDS },
        )
        .toBe(true);

    return present(found, "the copied row");
}

test.each(TEST_DIALECTS)(
    "apply secrets and selections at the vault service, let workloads read the versions their live deployments captured, and refuse destroying those versions on %s",
    async (dialect) => {
        const { database, kept, server, vaulted, binder, directory } = await serveCell(dialect);
        const system = (
            object: Parameters<typeof server.executeAsSystem>[0],
            name: string,
            call: SystemCall,
        ) => vaulted.objects.executeAsSystem(object, name, [call], Date.now());

        // submit and apply a stack revision as the stack controller does
        const read = async (): Promise<Installation> =>
            single(
                await database
                    .select()
                    .from(installation.table)
                    .where(eq(installation.table.id, ids.stack)),
            );
        const copiedType = (object: { readonly name: string }) =>
            present(
                server.sources
                    .flatMap((each) => each.objects)
                    .find((each) => each.name === object.name),
                `the copied ${object.name}`,
            );
        const apply = async (definition: ReturnType<typeof stack>) => {
            const current = await read();
            const revision = await database.transaction(async (transaction) =>
                recordSubmission(
                    transaction,
                    server.invoker({ database: transaction, scope: ids.space, now: Date.now() }),
                    current,
                    {
                        selection: { kind: "release", version: "2026.9.0" },
                        build,
                        evaluation: { export: "personal", parameters: {}, definition },
                    },
                    openBuild,
                ),
            );

            return applyStack(
                database,
                await read(),
                revision,
                [
                    serveInstallations({
                        resources: [vault],
                        declared: () => [],
                        directory,
                        home: async () => undefined,
                        openBuild,
                        binder,
                        runtimes: ["bun"],
                        cell: { regionId: ids.region },
                        consent: async (consent) => {
                            throw new TypeError(
                                `the stack records no consent of ${consent.userId}`,
                            );
                        },
                    }),
                    copiedType(vault),
                    copiedType(secret),
                    binder.binding,
                    ...Object.values(servePolicies()),
                    ...Object.values(serveRoles()),
                ],
                {
                    server,
                    directory,
                    release: async (packageId) => {
                        throw new TypeError(`the fixture opens no release of ${packageId}`);
                    },
                },
            );
        };

        // send the vault home and wait for its provisioned copy
        const waiting = await apply(stack());
        expect([waiting.steps.map((step) => [step.action, step.target]), waiting.deferred]).toEqual(
            [
                [
                    ["create", "installation/notes"],
                    ["create", "vault/credentials"],
                ],
                "vault credentials waits for its copy",
            ],
        );
        await copied(
            () => database.select().from(vault.table),
            (row) => row.reference !== null,
        );

        // send the secret home and bind its copy
        const declared = await apply(stack());
        await copied(
            () => database.select().from(secret.table),
            () => true,
        );
        const applied = await apply(stack());
        expect([
            declared.steps.map((step) => [step.action, step.target]),
            declared.deferred,
            applied.steps.map((step) => [step.action, step.target]),
        ]).toEqual([
            [["create", "secret/mail"]],
            "secret mail waits for its copy",
            [["create", `binding/installations/notes/${ids.notes}/token`]],
        ]);
        const mail = single(await kept.select().from(secret.table));
        const bound = single(await database.select().from(binder.binding.table));
        const notesInstallation = single(
            await database
                .select()
                .from(installation.table)
                .where(eq(installation.table.alias, "notes")),
        );
        const { revisionId } = notesInstallation;
        if (revisionId === null) {
            throw new TypeError("the notes installation has no revision");
        }
        expect([bound.name, bound.target]).toEqual(["token", mail.id]);

        // write two versions at home and wait for their copies
        for (const number of [1, 2]) {
            await system(secretVersion, "create", {
                scope: ids.space,
                input: {
                    parentId: mail.id,
                    value: { encoding: "text", value: `token ${number}` },
                },
            });
        }
        await copied(
            () => database.select().from(secret.table),
            (row) => row.currentVersion === 2,
        );
        const version = async (number: number) =>
            single(
                await kept
                    .select()
                    .from(secretVersion.table)
                    .where(eq(secretVersion.table.number, number)),
            );
        const promote = async (number: number) => {
            const row = single(await kept.select().from(secret.table));
            await system(secret, "promote", SystemCall.of(row, { version: number }));
            await copied(
                () => database.select().from(secret.table),
                (copy) => copy.currentVersion === number,
            );
        };

        // serve the vault's secrets to the notes installation
        const workload = principal.installation.reference(ids.space, notesInstallation.id);
        const hosted = Server.start({
            ...vaulted,
            controllers: [],
            audience: vaultService.package.id,
            resources: new ResourceContext(),
            health: new Health("vault"),
            authenticate: async () =>
                new Authentication({
                    credential: { kind: "fixture", id: "fixture-1" },
                    audience: vaultService.package.id,
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60_000,
                    subject: workload,
                    subjects: [workload],
                }),
            authorizeMachine: async () => {},
            drainTimeout: 1000,
        });
        onTestFinished(() => hosted.close());
        const client = new SecretClient(vaultService, {
            url: "https://vault.test",
            fetch: (request) => hosted.fetch(request),
        });
        const reading = { spaceId: ids.space, id: mail.id };
        const relate = () =>
            database.transaction((transaction) =>
                binder.relate(binderCall(server, transaction), ids.space, notesInstallation.id),
            );
        const deploy = async () => {
            const now = Date.now();
            const deployed = single(
                await database
                    .insert(deployment.table)
                    .values({
                        id: schema.identifier("deployment").parse(`deployment-${v7()}`),
                        scope: ids.space,
                        installationId: notesInstallation.id,
                        packageId: ids.notes,
                        revisionId: revisionId,
                        output: "main",
                        workload: "main",
                        runtime: "bun",
                        release: "2026.9.0",
                        description: {
                            entrypoint: ".",
                            services: [],
                            triggers: [],
                            resources: [],
                            secrets: [{ packageId: ids.notes, name: "token" }],
                            connections: [],
                            compute: {},
                            capabilities: {},
                            permissions: {},
                        },
                        policies: { packages: [], network: [] },
                        status: "active",
                        activatedAt: now,
                        createdAt: now,
                        updatedAt: now,
                    })
                    .returning(),
            );
            await database.transaction((transaction) =>
                binder.capture(binderCall(server, transaction), deployed),
            );
            await relate();

            return deployed;
        };
        const retire = async (retired: typeof deployment.table.$inferSelect) => {
            await database
                .update(deployment.table)
                .set({ status: "retired", retiredAt: Date.now() })
                .where(eq(deployment.table.id, retired.id));
            await relate();
        };
        const consumed = (isConsumed: boolean) =>
            expect
                .poll(
                    async () =>
                        (await kept.select().from(accessRelationship)).some(
                            (row) => row.relation === "consumer",
                        ),
                    { timeout: COPY_TIMEOUT_MILLISECONDS },
                )
                .toBe(isConsumed);
        const refused = new ServiceError("FORBIDDEN", {
            defined: true,
            message: "permission denied: read",
        });

        // read only the version a deployment captured
        const first = await deploy();
        await consumed(true);
        expect(await client.secret.read({ ...reading, version: 2 })).toEqual({
            version: 2,
            value: { encoding: "text", value: "token 2" },
        });
        await expect(client.secret.read(reading)).rejects.toEqual(refused);
        await expect(client.secret.read({ ...reading, version: 1 })).rejects.toEqual(refused);

        // refuse destroying the version a live deployment captured
        await copied(
            () => kept.select().from(deployment.table),
            (row) => row.id === first.id,
        );
        await expect(
            system(secretVersion, "destroy", SystemCall.of(await version(2))),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", {
                message: `secret version 2 is captured by ${first.id}`,
            }),
        );

        // keep reading the captured version after the secret's current one moves
        await promote(1);
        expect((await client.secret.read({ ...reading, version: 2 })).version).toBe(2);

        // read the fixed version once the old deployment retires and a new one captures it
        await apply(stack(1));
        await promote(2);
        await retire(first);
        const second = await deploy();
        await expect
            .poll(() => client.secret.read({ ...reading, version: 1 }).catch(() => undefined), {
                timeout: COPY_TIMEOUT_MILLISECONDS,
            })
            .toEqual({ version: 1, value: { encoding: "text", value: "token 1" } });
        await expect(client.secret.read({ ...reading, version: 2 })).rejects.toEqual(refused);

        // read nothing once no deployment is live
        await retire(second);
        await consumed(false);
        await expect(client.secret.read({ ...reading, version: 1 })).rejects.toEqual(
            new ServiceError("NOT_FOUND", { defined: true, message: `no scope ${ids.space}` }),
        );

        // destroy the fixed version once no live deployment captured it
        await copied(
            () => kept.select().from(deployment.table),
            (row) => row.id === second.id && row.status === "retired",
        );
        await system(secretVersion, "destroy", SystemCall.of(await version(1)));
        expect((await version(1)).destroyedAt).toEqual(expect.any(Number));

        // send the undeclared secret to the trash
        const removed = await apply(stack(undefined, false));
        expect(removed.steps.map((step) => [step.action, step.target])).toEqual([
            ["delete", `binding/installations/notes/${ids.notes}/token`],
            ["delete", "secret/mail"],
        ]);
        await copied(
            () => kept.select().from(secret.table),
            (row) => row.deletionRequestedAt !== null,
        );
    },
    30_000,
);

test("own the secret an installation's declaration generates in its vault, the vault service writing its ES256 key once its vault's key exists", async () => {
    // serve the space's objects and the vault service with notes installed
    const { database, kept, vaults, server, binder } = await serveCell("sqlite");
    const now = Date.now();
    const notesInstallation = single(
        await database
            .insert(installation.table)
            .values({
                id: schema
                    .identifier("installation")
                    .parse("installation-01996ab0-0000-7000-8000-000000000007"),
                scope: ids.space,
                packageId: ids.notes,
                role: "application",
                selection: { kind: "release", version: "2026.9.0" },
                alias: "notes",
                createdAt: now,
                updatedAt: now,
            })
            .returning(),
    );

    // need the vault and generated secret the notes build declares
    const needs = [
        { package: notes.package, name: "push", kind: VaultKind.name, spec: {}, state: {} },
        {
            package: notes.package,
            name: "push-key",
            kind: "secret",
            spec: { generated: { vault: "push", algorithm: "ES256" } },
            state: {},
        },
    ];
    const settle = () =>
        database.transaction(async (transaction) => {
            const call = binderCall(server, transaction);
            await binder.attach(call, notesInstallation, needs);

            return binder.ready(call, notesInstallation, needs);
        });

    // provision both and wait for the generated first version
    const waiting = await settle();
    await expect.poll(settle, { timeout: COPY_TIMEOUT_MILLISECONDS, interval: 200 }).toBe(true);
    const ready = await Promise.all([settle(), settle()]);
    const generated = single(await kept.select().from(secret.table));
    const key = await vaults.key(kept, generated.parentId);
    const version = single(await kept.select().from(secretVersion.table));
    const value = await SecretVersion.decrypt(key, generated, version);
    const jwk = schema
        .looseObject({ kty: schema.string(), crv: schema.string(), alg: schema.string() })
        .parse(JSON.parse(value.encoding === "text" ? value.value : "{}"));
    const bindings = await database
        .select()
        .from(binder.binding.table)
        .orderBy(binder.binding.table.name);
    const tables: readonly Table[] = [vault.table, secret.table, secretVersion.table];
    expect({
        waiting,
        ready,
        secret: [generated.name, generated.currentVersion, jwk.kty, jwk.crv, jwk.alg],
        bindings: bindings.map((row) => [row.name, row.target.split("-")[0]]),
        kept: (await Promise.all(tables.map((table) => kept.select().from(table)))).map(
            (rows) => rows.length,
        ),
    }).toEqual({
        waiting: false,
        ready: [true, true],
        secret: ["push-key", 1, "EC", "P-256", "ES256"],
        bindings: [
            ["push", "vault"],
            ["push-key", "secret"],
        ],
        kept: [1, 1, 1],
    });
}, 30_000);
