import { reconciliation, testJournalKey } from "@destack/service/test";
import { accessRelationship, principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { eq, type DatabaseConnection } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ObjectServer, SystemCall } from "@destack/object/server";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { Journal } from "@destack/service/database";
import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { defineSpace } from "@destack/space";
import { deployment, space, type Installation } from "@destack/space/object";
import {
    applyStack,
    Binder,
    binding,
    installation,
    networkPolicy,
    packagePolicy,
    ProviderIndex,
    recordSubmission,
    relationship,
    resourceBindable,
    role,
    serveResources,
    serveRevisions,
} from "@destack/space/server";
import { spaceObjects } from "@destack/space/service";
import { type PackageManifest, BuildReader } from "@destack/package/manifest";
import { v7 } from "uuid";
import { defineVault } from "../../declare/index.ts";
import { LocalKeyring } from "../../encryption/index.ts";
import { secretVersion, vault } from "../../object/index.ts";
import { connect } from "../../secret/client.ts";
import { implementService, secret, secretBindable, servedObjects } from "../index.ts";
import { vaultJournal } from "../../stack/index.ts";
import { cellTables } from "./fixture.ts";
import { vaultProvider } from "../../provider/index.ts";

/** The vault declared by the fixture stack. */
export const credentials = defineVault({ name: "credentials", spec: {} });

/** Identifiers of the fixture space and its stack. */
const ids = {
    account: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    stack: identifier("installation").parse("installation-01996ab0-0000-7000-8000-000000000003"),
    region: identifier("region").parse("region-01996ab0-0000-7000-8000-000000000004"),
    package: identifier("package").parse("package-01996ab0-0000-7000-8000-000000000005"),
    notes: identifier("package").parse("package-01996ab0-0000-7000-8000-000000000006"),
};

/** The notes installation with a mail token secret. */
const notes = {
    package: { id: ids.notes, name: "@example/notes", version: "2026.9.0" },
    status: "enabled" as const,
    alias: "notes",
    compute: {},
    tags: {},
};

/** A stack declaring a vault, a secret, and an application reading it, pinned to a version or not. */
function stack(pin?: number, isSecretDeclared = true) {
    const selection = {
        target: { type: "secret", name: "mail" },
        ...(pin === undefined ? {} : { version: pin }),
        state: {},
    };

    return defineSpace({
        resources: { credentials: { declaration: credentials, retention: "retain", tags: {} } },
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

/** Register the space, its scopes and its stack installation, as the space service does. */
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
        const opened = await TestDatabase.create(dialect, cellTables, { isMigrated: true });
        onTestFinished(() => opened.close());
        const database = opened.database;
        await register(database);

        // serve the space's objects except the space, and the vault's, to the system
        const keyring = await LocalKeyring.import(
            "one",
            new Map([["one", crypto.getRandomValues(new Uint8Array(32))]]),
        );
        const providers = new ProviderIndex({ regionId: ids.region }, [
            vaultProvider(database, keyring, "eu"),
        ]);
        const resources = serveResources(providers);
        const server = new ObjectServer({
            objects: {
                ...Object.fromEntries(
                    Object.entries(spaceObjects).filter(([name]) => name !== "space"),
                ),
                resource: resources,
                installationRevision: serveRevisions(
                    async (packageId) =>
                        new BuildReader(
                            { descriptions: {}, outputs: {} } as unknown as PackageManifest,
                            async (path) => {
                                throw new TypeError(
                                    `the fixture build of ${packageId} has no ${path}`,
                                );
                            },
                        ),
                ),
                ...servedObjects(keyring, "eu", { days: 1 }),
            },
            database,
            context: (context, scope) => context.access(scope),
            journal: new Journal(vaultJournal, testJournalKey),
            audit: AuditRecorder.service(new AuditOutbox(database), {
                package: vault.package,
                service: "vault",
            }),
        });
        const system = (
            object: Parameters<typeof server.executeAsSystem>[0],
            name: string,
            call: SystemCall,
        ) => server.executeAsSystem(object, name, [call], Date.now());

        // submit and apply a stack revision as the stack controller does
        const read = async (): Promise<Installation> =>
            (
                await database
                    .select()
                    .from(installation.table)
                    .where(eq(installation.table.id, ids.stack))
            )[0]!;
        const apply = async (definition: ReturnType<typeof stack>) => {
            const current = await read();
            const revision = await database.transaction(async (transaction) =>
                recordSubmission(
                    transaction,
                    (object, name, input) =>
                        server.invoke(transaction, ids.space, object, name, input, Date.now()),
                    current,
                    {
                        selection: { kind: "release", version: "2026.9.0" },
                        build,
                        evaluation: { export: "personal", parameters: {}, definition },
                    },
                ),
            );

            return applyStack(
                database,
                await read(),
                revision,
                [
                    installation,
                    resources,
                    binding,
                    networkPolicy,
                    packagePolicy,
                    role,
                    relationship,
                    secret,
                ],
                {
                    server,
                    release: async (packageId) => {
                        throw new TypeError(`the fixture opens no release of ${packageId}`);
                    },
                },
            );
        };

        // stop before secrets while the vault resource awaits provisioning
        const waiting = await apply(stack());
        expect(waiting.deferred).toBe("vault credentials is not provisioned");

        // provision the vault with its facet, then apply the secret and its selection
        const controller = server.controllers().find((each) => each.name === "resource")!;
        await controller.reconcile(
            JSON.stringify({ scope: ids.space }),
            reconciliation(new AbortController().signal),
        );
        const applied = await apply(stack());
        expect(applied.steps.map((step) => [step.action, step.target])).toEqual([
            ["create", "secret/mail"],
            ["create", `binding/installations/notes/${ids.notes}/token`],
        ]);
        const [mail] = await database.select().from(secret.table);
        const [bound] = await database.select().from(binding.table);
        const [notesInstallation] = await database
            .select()
            .from(installation.table)
            .where(eq(installation.table.alias, "notes"));
        expect([bound!.name, bound!.target]).toEqual(["token", mail!.id]);

        // write two versions of the secret, the second current
        for (const number of [1, 2]) {
            await system(secretVersion, "create", {
                scope: ids.space,
                input: {
                    parentId: mail!.id,
                    value: { encoding: "text", value: `token ${number}` },
                },
            });
        }
        const version = async (number: number) =>
            (
                await database
                    .select()
                    .from(secretVersion.table)
                    .where(eq(secretVersion.table.number, number))
            )[0]!;
        const promote = async (version: number) => {
            const [row] = await database.select().from(secret.table);
            await system(secret, "promote", SystemCall.of(row!, { version }));
        };

        // serve the vault to the notes installation
        const workload = principal.installation.reference(ids.space, notesInstallation!.id);
        const hosted = Server.start({
            ...implementService({
                journalKey: testJournalKey,
                database,
                keyring,
                location: "eu",
                recovery: { days: 1 },
            }),
            controllers: [],
            audience: vault.package.id,
            resources: new ResourceContext(),
            health: new Health("vault"),
            authenticate: async () =>
                new Caller({
                    credential: { kind: "fixture", id: "fixture-1" },
                    audience: vault.package.id,
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60_000,
                    subject: workload,
                    subjects: [workload],
                }),
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });
        onTestFinished(() => hosted.close());
        const client = connect({
            url: "https://vault.test",
            fetch: (request) => hosted.fetch(request),
        });
        const reading = { spaceId: ids.space, id: mail!.id };
        const binder = new Binder([resourceBindable, secretBindable]);
        const relate = () =>
            database.transaction((transaction) =>
                binder.relate(transaction, ids.space, notesInstallation!.id, Date.now()),
            );
        const deploy = async () => {
            const now = Date.now();
            const [deployed] = await database
                .insert(deployment.table)
                .values({
                    id: identifier("deployment").parse(`deployment-${v7()}`),
                    scope: ids.space,
                    installationId: notesInstallation!.id,
                    packageId: ids.notes,
                    revisionId: notesInstallation!.revisionId!,
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
                    },
                    policies: { packages: [], network: [] },
                    status: "active",
                    activatedAt: now,
                    createdAt: now,
                    updatedAt: now,
                })
                .returning();
            await database.transaction((transaction) =>
                binder.capture(
                    {
                        database: transaction,
                        invoke: (object, name, input) =>
                            server.invoke(transaction, ids.space, object, name, input, now),
                    },
                    deployed!,
                ),
            );
            await relate();

            return deployed!;
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

        // read the pinned version once the old deployment retires and a new one captures the pin
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
            new ServiceError("NOT_FOUND", { defined: true, message: `no scope ${ids.space}` }),
        );
        expect(await database.select().from(accessRelationship)).toEqual([]);

        // destroy the pinned version once no live deployment captured it
        await system(secretVersion, "destroy", SystemCall.of(await version(1)));
        expect((await version(1)).destroyedAt).toEqual(expect.any(Number));

        // move the undeclared secret to the trash
        const removed = await apply(stack(undefined, false));
        expect(removed.steps.map((step) => [step.action, step.target])).toEqual([
            ["delete", `binding/installations/notes/${ids.notes}/token`],
            ["delete", "secret/mail"],
        ]);
        const [trashed] = await database.select().from(secret.table);
        expect(trashed!.deletionRequestedAt).toEqual(expect.any(Number));
    },
);
