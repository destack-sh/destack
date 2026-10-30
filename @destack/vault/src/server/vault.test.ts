import { Account } from "@destack/account/object";
import { type Table } from "@destack/db";
import { type Identifier } from "@destack/schema";
import { Package } from "@destack/package";
import { ids, TestSpace } from "@destack/stack/test";
import { RequestId } from "@destack/sync";
import { expect, refusal, single, test } from "@destack/test";
import { defineSecret, defineVault } from "../declare/index.ts";
import { secret, secretVersion, vault } from "../object/index.ts";
import vaultPackage from "../package.ts";
import vaultServer from "../server.ts";
import { APPLICATION, CONSUMER, CREDENTIAL, CREDENTIALS, TestVault } from "../test/index.ts";

/** A vault the space's stack declares beside the application's, deleted at once on removal. */
const shared = defineVault({ name: "shared", spec: {} });

/** A package whose installation binds nothing. */
const STRANGER = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c3",
    name: "@example/stranger",
    version: "2026.10.0",
});

/** The consumer's vault, bound to a vault of another installation. */
const CONSUMER_VAULT = defineVault({ name: CREDENTIALS, spec: {} }, { package: CONSUMER });

/** The consumer's secret, bound to a secret of another installation. */
const CONSUMER_SECRET = defineSecret({ name: "token" }, { package: CONSUMER });

/** Install the consumer bound to an existing vault or secret, answering its installation. */
async function installConsumer(
    fixture: TestVault,
    target: { readonly type: "vault" | "secret"; readonly id: string },
): Promise<Identifier<"installation">> {
    // declare the vault or the secret the consumer binds, by the target's type
    const isVault = target.type === "vault";
    const build = await TestVault.build(
        CONSUMER,
        isVault ? [CONSUMER_VAULT] : [],
        isVault ? [] : [CONSUMER_SECRET],
        fixture.environment.crypto,
    );
    const name = isVault ? CONSUMER_VAULT.name : CONSUMER_SECRET.name;

    return fixture.space.install(fixture.served, build, {
        bindings: { [CONSUMER.id]: { [name]: { target } } },
    });
}

/** Put a value of a secret named token through a vault as an installation. */
function put(
    fixture: TestVault,
    installation: Identifier<"installation">,
    target: { readonly vault: Identifier<"vault">; readonly secret: Identifier<"secret"> },
    value: string,
): Promise<void> {
    return fixture.mutate(
        installation,
        vault.call("put", {
            ...vault.target({ scope: ids.space, id: target.vault }),
            secret: target.secret,
            name: "token",
            value: { encoding: "text", value },
        }),
    );
}

test("read a vault's secret by name, never a secret of the same name in another vault", async () => {
    // keep a secret of the same name in the credentials vault and another one
    const fixture = await TestVault.open({
        vaults: [TestVault.declare(CREDENTIALS), TestVault.declare("other")],
    });
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    await fixture.createSecret(caller);
    const credentials = await fixture.vault();
    const other = await fixture.vault("other");
    const created = await client.secret.create({
        scope: ids.space,
        parentId: other,
        name: "first",
        requestId: RequestId.create(fixture.environment),
    });
    await client.version.create({
        scope: ids.space,
        parentId: created.id,
        requestId: RequestId.create(fixture.environment),
        value: { encoding: "text", value: "other credential" },
    });
    const read = (id: Identifier<"vault">, name: string) =>
        client.vault.read({ scope: ids.space, id, name, version: 1 });

    // read each vault's own secret, and nothing the vault lacks
    expect({
        own: await read(credentials, "first"),
        another: await read(other, "first"),
        missing: await refusal(read(credentials, "second")),
    }).toEqual({
        own: { version: 1, value: CREDENTIAL },
        another: { version: 1, value: { encoding: "text", value: "other credential" } },
        missing: ["NOT_FOUND", "SECRET_NOT_FOUND", "vault has no secret second"],
    });
});

test("let an installation bound to a vault read its secrets by name without a step-up, and conceal the vault from another", async () => {
    // bind the consumer to the application's vault, and install a stranger binding nothing
    const fixture = await TestVault.open();
    await fixture.createSecret(fixture.owner);
    const credentials = await fixture.vault();
    const consumer = await installConsumer(fixture, { type: "vault", id: credentials });
    const stranger = await fixture.space.install(
        fixture.served,
        await TestVault.build(STRANGER, [], [], fixture.environment.crypto),
    );

    // read the secret by name as each installation
    const read = (installation: Identifier<"installation">) =>
        fixture
            .installed(installation)
            .client(vaultPackage)
            .vault.read({ scope: ids.space, id: credentials, name: "first" });
    expect([await read(consumer), await refusal(read(stranger))]).toEqual([
        { version: 1, value: CREDENTIAL },
        ["NOT_FOUND", "NOT_FOUND", `vault ${credentials} not found`],
    ]);
});

test("put a consumer's secret as one live version through its vault, refuse another name, read it as the consumer, conceal it from a stranger, and remove it", async () => {
    // put twice through the vault as the application binding it, at a paused time
    const fixture = await TestVault.open();
    const stranger = await fixture.space.install(
        fixture.served,
        await TestVault.build(STRANGER, [], [], fixture.environment.crypto),
    );
    const target = { vault: await fixture.vault(), secret: secret.generateId(fixture.environment) };
    const time = fixture.clock.now();
    await put(fixture, fixture.installation, target, "first");
    await put(fixture, fixture.installation, target, "second");

    // refuse putting the secret again under another name
    const renamed = await refusal(
        fixture.mutate(
            fixture.installation,
            vault.call("put", {
                ...vault.target({ scope: ids.space, id: target.vault }),
                secret: target.secret,
                name: "other",
                value: { encoding: "text", value: "third" },
            }),
        ),
    );

    // read the live value as the application, refusing the stranger
    const read = (installation: Identifier<"installation">) =>
        fixture
            .installed(installation)
            .client(vaultPackage)
            .secret.read({ scope: ids.space, id: target.secret });
    const live = await read(fixture.installation);
    const concealed = await refusal(read(stranger));
    const versions = await fixture.served.database
        .select({
            number: secretVersion.table.number,
            state: secretVersion.table.state,
        })
        .from(secretVersion.table)
        .orderBy(secretVersion.table.number);

    // remove it with its values
    await fixture.mutate(
        fixture.installation,
        vault.call("remove", {
            ...vault.target({ scope: ids.space, id: target.vault }),
            secret: target.secret,
        }),
    );
    const removed = await fixture.served.database
        .select({
            name: secret.table.name,
            deletedAt: secret.table.deletedAt,
            purgedAt: secret.table.purgedAt,
        })
        .from(secret.table);
    expect({ renamed, live, concealed, versions, removed }).toEqual({
        renamed: [
            "CONFLICT",
            "SECRET_MISMATCH",
            `secret ${target.secret} is token in this vault, not other`,
        ],
        live: { version: 2, value: { encoding: "text", value: "second" } },
        concealed: ["NOT_FOUND", "NOT_FOUND", `secret ${target.secret} not found`],
        versions: [
            { number: 1, state: "destroyed" },
            { number: 2, state: "enabled" },
        ],
        removed: [{ name: "token", deletedAt: time, purgedAt: time }],
    });
});

test("let installations bound to a vault put and rotate their own secrets there, refusing another consumer their replacement and an installation bound to one secret the whole vault", async () => {
    // bind the consumer to the application's vault, and a stranger's installation to one of its secrets
    const fixture = await TestVault.open();
    const credentials = await fixture.vault();
    const declared = await fixture.createSecret(fixture.owner);
    const other = await installConsumer(fixture, { type: "vault", id: credentials });
    const reader = await fixture.space.install(
        fixture.served,
        await TestVault.build(
            STRANGER,
            [],
            [defineSecret({ name: "token" }, { package: STRANGER })],
            fixture.environment.crypto,
        ),
        {
            bindings: {
                [STRANGER.id]: {
                    token: { target: { type: "secret", id: declared.key.id } },
                },
            },
        },
    );

    // put and rotate the application's own secret, and refuse the others replacing, removing or planting one
    const target = { vault: credentials, secret: secret.generateId(fixture.environment) };
    const read = () =>
        fixture
            .installed(fixture.installation)
            .client(vaultPackage)
            .secret.read({ scope: ids.space, id: target.secret });
    await put(fixture, fixture.installation, target, "first");
    await put(fixture, fixture.installation, target, "second");
    const rotated = await read();
    const replaced = await refusal(put(fixture, other, target, "stolen"));
    const removed = await refusal(
        fixture.mutate(
            other,
            vault.call("remove", {
                ...vault.target({ scope: ids.space, id: credentials }),
                secret: target.secret,
            }),
        ),
    );
    const planted = await refusal(
        put(
            fixture,
            reader,
            { vault: credentials, secret: secret.generateId(fixture.environment) },
            "planted",
        ),
    );
    expect({ rotated, replaced, removed, planted, after: await read() }).toEqual({
        rotated: { version: 2, value: { encoding: "text", value: "second" } },
        replaced: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: update"],
        removed: ["NOT_FOUND", "NOT_FOUND", `secret ${target.secret} not found`],
        planted: ["NOT_FOUND", "NOT_FOUND", `vault ${credentials} not found`],
        after: { version: 2, value: { encoding: "text", value: "second" } },
    });
});

test("destroy a vault's secrets with their records on its removal from the stack", async () => {
    // keep a secret in a vault the space's stack declares, deleted at once on removal
    const fixture = await TestVault.open({ vaults: [] });
    await fixture.space.apply(fixture.served, [vaultPackage], {
        vaults: { shared: shared.declare({ retention: "delete" }) },
    });
    const { database } = fixture.served;
    const provisioned = single(await database.select({ id: vault.table.id }).from(vault.table));
    const client = fixture.owner.client(vaultPackage);
    const created = await client.secret.create({
        scope: ids.space,
        parentId: provisioned.id,
        name: "token",
        requestId: RequestId.create(fixture.environment),
    });
    await client.version.create({
        scope: ids.space,
        parentId: created.id,
        requestId: RequestId.create(fixture.environment),
        value: CREDENTIAL,
    });

    // remove the vault from the stack, destroying its secret and its versions with it
    await fixture.space.apply(fixture.served, [vaultPackage], { vaults: {} });
    const remaining = await Promise.all(
        [vault.table, secret.table, secretVersion.table].map((table: Table) =>
            database.select().from(table),
        ),
    );
    expect(remaining).toEqual([[], [], []]);
});

test("refuse removing a vault while an installation binds one of its secrets", async () => {
    // keep a secret in a vault the space's stack declares, bound by the consumer's installation
    const fixture = await TestVault.open({ vaults: [] });
    await fixture.space.apply(fixture.served, [vaultPackage], {
        vaults: { shared: shared.declare({ retention: "delete" }) },
    });
    const provisioned = single(await fixture.served.database.select().from(vault.table));
    const token = await fixture.owner.client(vaultPackage).secret.create({
        scope: ids.space,
        parentId: provisioned.id,
        name: "token",
        requestId: RequestId.create(fixture.environment),
    });
    const consumer = await installConsumer(fixture, { type: "secret", id: token.id });

    // refuse the vault's removal, the stack reporting why, keeping the vault and its secret
    const refused = await TestSpace.refusals(
        fixture.space.apply(fixture.served, [vaultPackage], { vaults: {} }),
    );
    expect({
        refused,
        kept: [
            await fixture.served.database
                .select({ deletedAt: vault.table.deletedAt })
                .from(vault.table),
            await fixture.served.database
                .select({ deletedAt: secret.table.deletedAt })
                .from(secret.table),
        ],
    }).toEqual({
        refused: [["CONFLICT", "BOUND", `${token.id} is in use: bound by ${consumer}`]],
        kept: [[{ deletedAt: null }], [{ deletedAt: null }]],
    });
});

test("refuse removing a vault while an installation binds the vault itself", async () => {
    // bind the consumer to a vault the space's stack declares
    const fixture = await TestVault.open({ vaults: [] });
    await fixture.space.apply(fixture.served, [vaultPackage], {
        vaults: { shared: shared.declare({ retention: "delete" }) },
    });
    const provisioned = single(await fixture.served.database.select().from(vault.table));
    const consumer = await installConsumer(fixture, { type: "vault", id: provisioned.id });

    // refuse the vault's removal, the stack reporting why, keeping the vault
    const refused = await TestSpace.refusals(
        fixture.space.apply(fixture.served, [vaultPackage], { vaults: {} }),
    );
    expect({
        refused,
        kept: await fixture.served.database
            .select({ deletedAt: vault.table.deletedAt })
            .from(vault.table),
    }).toEqual({
        refused: [["CONFLICT", "BOUND", `${provisioned.id} is in use: bound by ${consumer}`]],
        kept: [{ deletedAt: null }],
    });
});

test("trash a vault's secrets with it once its installation goes", async () => {
    // install the application with a key in its vault, then remove the installation at a paused time
    const fixture = await TestVault.open({
        secrets: [
            defineSecret(
                { name: "key", generated: { vault: CREDENTIALS, algorithm: "ES256" } },
                { package: APPLICATION },
            ),
        ],
    });
    const removedAt = fixture.clock.now();
    await fixture.space.apply(fixture.served, [], { installations: {} });

    // trash the vault and its key in the same request
    const { database } = fixture.served;
    expect([
        await database.select({ deletedAt: vault.table.deletedAt }).from(vault.table),
        await database.select({ deletedAt: secret.table.deletedAt }).from(secret.table),
    ]).toEqual([[{ deletedAt: removedAt }], [{ deletedAt: removedAt }]]);
});

test("provision a vault the account's home declares in the account, as its connections keep their credentials there", async () => {
    // serve the account's home with the vault package, and declare a vault in its account
    const home = await TestSpace.open({ id: Account.home(ids.account), packages: [vaultPackage] });
    const served = await home.serve({ servers: [vaultServer] });
    const credentials = defineVault({ name: "credentials", spec: {} });
    await home.apply(served, [vaultPackage], {
        vaults: { credentials: { ...credentials.declare(), scope: "enclosing" } },
    });

    // store the vault in the account, below the home
    const rows = await served.database
        .select({ scope: vault.table.scope, name: vault.table.name })
        .from(vault.table);
    expect(rows).toEqual([{ scope: ids.account, name: "credentials" }]);
});
