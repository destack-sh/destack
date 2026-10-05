import { reconciliation, testCallKey } from "@destack/service/test";
import { VaultKind } from "../declare/kind.ts";
import { accessRelationship, principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import { expect, onTestFinished, single, test } from "@destack/test";
import { eq, type DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { DirectoryStore, directoryTables, ZONE_SCOPE, zoneTable } from "@destack/directory";
import { ObjectServer, SystemAuthorization, SystemCall } from "@destack/object/server";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";

import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { defineSpace } from "@destack/space";
import { deployment, installation, space, type Installation } from "@destack/space/object";
import {
    applyStack,
    Bindable,
    Binder,
    type OpenBuild,
    ProviderIndex,
    recordSubmission,
    serveInstallations,
    servePolicies,
    serveResources,
    serveRevisions,
    serveRoles,
} from "@destack/space/server";
import { spaceObjects } from "@destack/space/service";
import { PackageFile } from "@destack/package/file";
import { PackageManifest, BuildReader } from "@destack/package/manifest";
import { v7 } from "uuid";
import { defineVault } from "../declare/index.ts";
import { LocalKeyring } from "@destack/host/keychain";
import { SecretVersion, secretVersion, vault } from "../object/index.ts";
import { VaultKey } from "../encryption/index.ts";
import { SecretClient } from "../object/index.ts";
import { spaceService } from "@destack/space/service";
import { secret, secretBindable, serveSecrets } from "./index.ts";
import { spaceDatabase } from "@destack/space/stack";
import { KeyringVaultHost, vaultProvider } from "../provider/index.ts";

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

/** The empty file every fixture build lists as its dependencies, files and source maps. */
const emptyFile = await PackageFile.describe(
    "manifest/empty.json",
    "application/json",
    new Uint8Array(),
);

/** The graph root of a build without modules. */
const emptyGraph = new TextEncoder().encode(JSON.stringify({ modules: {} }));

/** The file holding the empty graph root. */
const emptyGraphFile = await PackageFile.describe(
    "manifest/graph.json",
    "application/json",
    emptyGraph,
);

/** Open every build as an empty build of its package. */
const openBuild: OpenBuild = async (packageId) =>
    new BuildReader(
        PackageManifest.parse({
            formatVersion: 1,
            package: { id: packageId, name: "@example/fixture", version: "2026.9.0" },
            language: "typescript",
            lists: {
                dependencies: emptyFile,
                files: emptyFile,
                sourceMaps: emptyFile,
                graph: emptyGraphFile,
            },
            outputs: {},
        }),
        async (path) => {
            if (path === emptyGraphFile.path) {
                return emptyGraph;
            }
            throw new TypeError(`the fixture build of ${packageId} has no ${path}`);
        },
    );

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
    await copyScope(database, account.reference(Scope.universe.id, ids.account));
    await copyScope(database, space.reference(ids.account, ids.space));
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

test.each(TEST_DIALECTS)(
    "apply secrets and selections, let workloads read the versions their live deployments captured, and refuse destroying those versions on %s",
    async (dialect) => {
        const opened = await TestDatabase.create(dialect, spaceDatabase, { isMigrated: true });
        onTestFinished(() => opened.close());
        const database = opened.database;
        const universe = await TestDatabase.create("sqlite", directoryTables, { isMigrated: true });
        onTestFinished(() => universe.close());
        const directory = new DirectoryStore(universe.database);
        await register(database);

        // serve the space's objects except the space, and the vault's, to the system
        const keyring = await LocalKeyring.import(
            "one",
            new Map([["one", crypto.getRandomValues(new Uint8Array(32))]]),
        );
        const providers = new ProviderIndex({ regionId: ids.region }, [
            vaultProvider(new KeyringVaultHost(database, keyring, "eu")),
        ]);
        const resources = serveResources(providers);
        const secrets = serveSecrets(keyring, "eu", { days: 1 });
        const binder = new Binder([Bindable.resource(VaultKind, vault)], [secretBindable]);
        const server = new ObjectServer({
            objects: {
                ...Object.fromEntries(
                    Object.entries(spaceObjects).filter(
                        ([name]) => name !== "space" && name !== "connection",
                    ),
                ),
                ...resources,
                ...secrets,
                installationRevision: serveRevisions(openBuild),
            },
            directory,
            database,
            callKey: testCallKey,
            origin: {
                package: spaceService.package,
                service: spaceService.name,
            },
        });
        const system = (
            object: Parameters<typeof server.executeAsSystem>[0],
            name: string,
            call: SystemCall,
        ) => server.executeAsSystem(object, name, [call], Date.now());

        // submit and apply a stack revision as the stack controller does
        const read = async (): Promise<Installation> =>
            single(
                await database
                    .select()
                    .from(installation.table)
                    .where(eq(installation.table.id, ids.stack)),
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
                        resources: providers.objects(),
                        declared: () => [],
                        directory,
                        home: async () => undefined,
                        openBuild,
                        binder,
                        runtimes: ["bun"],
                        cell: { regionId: ids.region },
                    }),
                    ...Object.values(resources),
                    secrets.secret,
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

        // stop before secrets while the vault awaits provisioning
        const waiting = await apply(stack());
        expect(waiting.deferred).toBe("vault credentials is not provisioned");

        // provision the vault, then apply the secret and its selection
        const controller = single(server.controllers().filter((each) => each.name === "vault"));
        await controller.reconcile(
            JSON.stringify({ scope: ids.space }),
            reconciliation(new AbortController().signal),
        );
        const applied = await apply(stack());
        expect(applied.steps.map((step) => [step.action, step.target])).toEqual([
            ["create", "secret/mail"],
            ["create", `binding/installations/notes/${ids.notes}/token`],
        ]);
        const mail = single(await database.select().from(secret.table));
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

        // write two versions of the secret, the second current
        for (const number of [1, 2]) {
            await system(secretVersion, "create", {
                scope: ids.space,
                input: {
                    parentId: mail.id,
                    value: { encoding: "text", value: `token ${number}` },
                },
            });
        }
        const version = async (number: number) =>
            single(
                await database
                    .select()
                    .from(secretVersion.table)
                    .where(eq(secretVersion.table.number, number)),
            );
        const promote = async (number: number) => {
            const row = single(await database.select().from(secret.table));
            await system(secret, "promote", SystemCall.of(row, { version: number }));
        };

        // serve the space's secrets to the notes installation
        const workload = principal.installation.reference(ids.space, notesInstallation.id);
        const hosted = Server.start({
            ...server.implement(spaceService),
            controllers: [],
            audience: spaceService.package.id,
            resources: new ResourceContext(),
            health: new Health("space"),
            authenticate: async () =>
                new Authentication({
                    credential: { kind: "fixture", id: "fixture-1" },
                    audience: spaceService.package.id,
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60_000,
                    subject: workload,
                    subjects: [workload],
                }),
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });
        onTestFinished(() => hosted.close());
        const client = new SecretClient(spaceService, {
            url: "https://vault.test",
            fetch: (request) => hosted.fetch(request),
        });
        const reading = { spaceId: ids.space, id: mail.id };
        const relate = () =>
            database.transaction(async (transaction) =>
                binder.relate(
                    await SystemAuthorization.open(
                        server.authorizer,
                        transaction,
                        ids.space,
                        Date.now(),
                    ),
                    ids.space,
                    notesInstallation.id,
                ),
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
                binder.capture(
                    {
                        database: transaction,
                        invoke: server.invoker({ database: transaction, scope: ids.space, now }),
                    },
                    deployed,
                ),
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
        const refused = new ServiceError("FORBIDDEN", {
            defined: true,
            message: "permission denied: read",
        });

        // read the version a deployment captured from an unpinned selection, and no other
        const first = await deploy();
        expect(await client.secret.read({ ...reading, version: 2 })).toEqual({
            version: 2,
            value: { encoding: "text", value: "token 2" },
        });
        await expect(client.secret.read(reading)).rejects.toEqual(refused);
        await expect(client.secret.read({ ...reading, version: 1 })).rejects.toEqual(refused);

        // refuse destroying the captured version while its deployment is live
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
        expect(await client.secret.read({ ...reading, version: 1 })).toEqual({
            version: 1,
            value: { encoding: "text", value: "token 1" },
        });
        await expect(client.secret.read({ ...reading, version: 2 })).rejects.toEqual(refused);

        // read nothing once no deployment is live
        await retire(second);
        await expect(client.secret.read({ ...reading, version: 1 })).rejects.toEqual(
            new ServiceError("NOT_FOUND", { defined: true, message: `no secret ${mail.id}` }),
        );
        expect(await database.select().from(accessRelationship)).toEqual([]);

        // destroy the fixed version once no live deployment captured it
        await system(secretVersion, "destroy", SystemCall.of(await version(1)));
        expect((await version(1)).destroyedAt).toEqual(expect.any(Number));

        // move the undeclared secret to the trash
        const removed = await apply(stack(undefined, false));
        expect(removed.steps.map((step) => [step.action, step.target])).toEqual([
            ["delete", `binding/installations/notes/${ids.notes}/token`],
            ["delete", "secret/mail"],
        ]);
        const trashed = single(await database.select().from(secret.table));
        expect(trashed.deletionRequestedAt).toEqual(expect.any(Number));
    },
);

test("own the secret an installation's declaration generates in its vault, writing its ES256 key once when the vault's key exists", async () => {
    // serve the space's objects and the vault's to the system, with the notes application installed
    const opened = await TestDatabase.create("sqlite", spaceDatabase, { isMigrated: true });
    onTestFinished(() => opened.close());
    const database = opened.database;
    const universe = await TestDatabase.create("sqlite", directoryTables, { isMigrated: true });
    onTestFinished(() => universe.close());
    await register(database);
    const keyring = await LocalKeyring.import(
        "one",
        new Map([["one", crypto.getRandomValues(new Uint8Array(32))]]),
    );
    const providers = new ProviderIndex({ regionId: ids.region }, [
        vaultProvider(new KeyringVaultHost(database, keyring, "eu")),
    ]);
    const binder = new Binder([Bindable.resource(VaultKind, vault)], [secretBindable]);
    const server = new ObjectServer({
        objects: {
            ...Object.fromEntries(
                Object.entries(spaceObjects).filter(
                    ([name]) => name !== "space" && name !== "connection",
                ),
            ),
            ...serveResources(providers),
            ...serveSecrets(keyring, "eu", { days: 1 }),
        },
        directory: new DirectoryStore(universe.database),
        database,
        callKey: testCallKey,
        origin: { package: spaceService.package, service: spaceService.name },
    });
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

    // need a vault and a secret it generates, as the notes build declares them
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
            const call = {
                database: transaction,
                invoke: server.invoker({
                    database: transaction,
                    scope: ids.space,
                    now: Date.now(),
                }),
            };
            await binder.attach(call, notesInstallation, needs);

            return binder.ready(call, notesInstallation, needs);
        });

    // own both, waiting for the vault's key, then write the key once from two settles racing for it
    const waiting = await settle();
    const controller = single(server.controllers().filter((each) => each.name === "vault"));
    await controller.reconcile(
        JSON.stringify({ scope: ids.space }),
        reconciliation(new AbortController().signal),
    );
    const ready = await Promise.all([settle(), settle()]);
    const generated = single(await database.select().from(secret.table));
    const key = await VaultKey.load(database, keyring, "eu", generated.parentId);
    const version = single(await database.select().from(secretVersion.table));
    const value = await SecretVersion.open(key, generated, version);
    const jwk = schema
        .looseObject({ kty: schema.string(), crv: schema.string(), alg: schema.string() })
        .parse(JSON.parse(value.encoding === "text" ? value.value : "{}"));
    const bindings = await database
        .select()
        .from(binder.binding.table)
        .orderBy(binder.binding.table.name);
    expect({
        waiting,
        ready,
        secret: [generated.name, generated.currentVersion, jwk.kty, jwk.crv, jwk.alg],
        bindings: bindings.map((row) => [row.name, row.target.split("-")[0]]),
    }).toEqual({
        waiting: false,
        ready: [true, true],
        secret: ["push-key", 1, "EC", "P-256", "ES256"],
        bindings: [
            ["push", "vault"],
            ["push-key", "secret"],
        ],
    });
});
