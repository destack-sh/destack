import { eq, Snapshot, type DatabaseConnection } from "@destack/db";
import { CallEvent, call } from "@destack/object";
import { Duration, type Identifier, schema } from "@destack/schema";
import { Binding, Rollout, rollout } from "@destack/stack/object";
import { ids, TestSpace } from "@destack/stack/test";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { principal } from "@destack/access";
import { expect, refusal, single, test } from "@destack/test";
import { defineSecret, defineVault } from "../declare/index.ts";
import { SECRET_RECOVERY, secret, secretVersion } from "../object/index.ts";
import { SecretVersion } from "./version.ts";
import vaultPackage from "../package.ts";
import {
    APPLICATION,
    CONSUMER,
    CREDENTIAL,
    CREDENTIALS,
    TestVault,
    VAULT_PERMISSIONS,
} from "../test/index.ts";
import { secretBindable } from "./secret.ts";

/** The recovery window a machine gives deleted secrets, in milliseconds. */
const RECOVERY_MILLISECONDS = Duration.milliseconds(SECRET_RECOVERY);

/** The space, which purges secrets when their window ends and applies its stack. */
const SPACE = principal.space.reference(Scope.universe.id, ids.space);

/** The fixture's first version as a read answers it. */
const FIRST = { version: 1, value: CREDENTIAL };

/** Every vault permission but the reveal of secrets. */
const CONCEALING = VAULT_PERMISSIONS.filter(
    (permission) => permission.type !== secret.name || permission.name !== "reveal",
);

/** A vault the space's stack declares, deleted at once on removal. */
const SHARED = defineVault({ name: "shared" });

/** The application's key, which the space generates in its credentials vault. */
const KEY = defineSecret(
    { name: "key", generated: { vault: CREDENTIALS, algorithm: "ES256" } },
    { package: APPLICATION },
);

test("roundtrip bounded text and binary values without committing rejected writes", async () => {
    const fixture = await TestVault.open();
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    const created = await client.secret.create({
        scope: ids.space,
        parentId: await fixture.vault(),
        name: "binary",
        requestId: RequestId.create(fixture.environment),
    });
    const key = { scope: ids.space, id: created.id };
    const write = (value: { readonly encoding: "text" | "base64"; readonly value: string }) =>
        client.version.create({
            scope: ids.space,
            parentId: created.id,
            requestId: RequestId.create(fixture.environment),
            value,
        });

    // write the largest binary value, and refuse one byte more in either encoding
    const binary = {
        encoding: "base64" as const,
        value: Uint8Array.from({ length: 65536 }, (_, index) => index % 256).toBase64(),
    };
    const written = await write(binary);
    const refused = [
        await refusal(write({ encoding: "base64", value: new Uint8Array(65537).toBase64() })),
        await refusal(write({ encoding: "text", value: "é".repeat(32769) })),
    ];
    const listed = await client.version.list({ scope: ids.space });

    // write the largest UTF-8 text as current, keeping the binary version readable
    const text = { encoding: "text" as const, value: "é".repeat(32768) };
    const textual = await write(text);
    await fixture.point(caller, key, textual.number);
    expect({
        refused,
        listed: listed.items,
        current: await client.secret.read(key),
        first: await client.secret.read({ ...key, version: 1 }),
    }).toEqual({
        refused: [
            ["BAD_REQUEST", "VALUE_INVALID", "secret value is invalid: it exceeds 65536 bytes"],
            ["BAD_REQUEST", "VALUE_INVALID", "secret value is invalid: it exceeds 65536 bytes"],
        ],
        listed: [written],
        current: { version: 2, value: text },
        first: { version: 1, value: binary },
    });
});

test("add a version, point the current alias at it, destroy and restore, recording each read's version alone", async () => {
    const fixture = await TestVault.open();
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    const time = fixture.clock.now();
    const { sequence } = await fixture.served.database.log.position();
    const { secret: created, key, version: written } = await fixture.createSecret(caller);

    // add a second version, then point the current alias at it
    const added = await client.version.create({
        scope: ids.space,
        parentId: key.id,
        requestId: RequestId.create(fixture.environment),
        value: { encoding: "base64", value: "AAECAw==" },
    });
    const before = await client.secret.read(key);
    const pointed = await fixture.point(caller, key, added.number);
    const after = await client.secret.read(key);

    // delete the secret, and read it again once restored
    await client.secret.delete({
        ...key,
        requestId: RequestId.create(fixture.environment),
        revision: pointed.revision,
    });
    await client.secret.restore({ ...key, requestId: RequestId.create(fixture.environment) });
    const restored = await client.secret.read(key);

    // destroy the first version, refusing its value and any change to it
    const destroyed = await client.version.destroy({
        scope: ids.space,
        id: written.id,
        requestId: RequestId.create(fixture.environment),
    });
    const unavailable = await refusal(client.secret.read({ ...key, version: 1 }));
    const disabled = await refusal(
        client.version.disable({
            scope: ids.space,
            id: written.id,
            requestId: RequestId.create(fixture.environment),
        }),
    );

    // refuse pointing the current alias at a missing or destroyed version, or unsetting it
    const unset = () =>
        client.secret.update({
            ...key,
            requestId: RequestId.create(fixture.environment),
            currentVersion: null,
        });
    const pointing = [
        await refusal(fixture.point(caller, key, 9)),
        await refusal(fixture.point(caller, key, 1)),
        await refusal(unset()),
    ];
    expect({
        before,
        pointed,
        after,
        restored,
        destroyed,
        unavailable,
        disabled,
        pointing,
    }).toEqual({
        before: FIRST,
        pointed: { ...created, currentVersion: 2, revision: 3, updatedAt: time },
        after: { version: 2, value: { encoding: "base64", value: "AAECAw==" } },
        restored: { version: 2, value: { encoding: "base64", value: "AAECAw==" } },
        destroyed: { ...written, state: "destroyed", revision: 2, updatedAt: time },
        unavailable: [
            "PRECONDITION_FAILED",
            "VERSION_BLOCKED",
            "cannot read secret version: it is destroyed, disabled or expired",
        ],
        disabled: [
            "PRECONDITION_FAILED",
            "VERSION_BLOCKED",
            "cannot change secret version: it is destroyed",
        ],
        pointing: [
            ["NOT_FOUND", "VERSION_NOT_FOUND", "secret version not found"],
            [
                "PRECONDITION_FAILED",
                "VERSION_BLOCKED",
                "cannot read secret version: it is destroyed, disabled or expired",
            ],
            [
                "BAD_REQUEST",
                "CURRENT_VERSION_REQUIRED",
                "current version is required: a secret's current version cannot be unset",
            ],
        ],
    });

    // record the version each read disclosed, and no value, in commit order
    const { events } = await fixture.served.store.events.changes(
        call.eventKind(),
        { scopes: [ids.space] },
        sequence,
    );
    const reads = events
        .map((event) => CallEvent.parse(event.data))
        .filter((ended) => ended.method === "secret.read")
        .map((ended) => [ended.execution.category, ended.execution.details]);
    expect(reads).toEqual([
        ["access", { version: 1 }],
        ["access", { version: 2 }],
        ["access", { version: 2 }],
        ["access", {}],
    ]);
});

test("refuse writing a version with a past expiry and reading an expired version", async () => {
    const fixture = await TestVault.open();
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    const { key, request } = await fixture.createSecret(caller);

    // refuse a version that expires before it is written, and read one before its expiry
    const past = await refusal(
        client.version.create({
            ...request,
            requestId: RequestId.create(fixture.environment),
            expiresAt: fixture.clock.now() - 1000,
        }),
    );
    const expiresAt = fixture.clock.now() + 60_000;
    const expiring = await client.version.create({
        ...request,
        requestId: RequestId.create(fixture.environment),
        expiresAt,
    });
    await fixture.point(caller, key, expiring.number);
    const live = await client.secret.read(key);

    // refuse reading the version once its expiry passed
    await fixture.space.simulation.run(expiresAt);
    expect([past, live, await refusal(client.secret.read(key))]).toEqual([
        [
            "BAD_REQUEST",
            "EXPIRY_IN_PAST_INVALID",
            "secret expiry is invalid: it must be in the future",
        ],
        { version: 2, value: CREDENTIAL },
        [
            "PRECONDITION_FAILED",
            "VERSION_BLOCKED",
            "cannot read secret version: it is destroyed, disabled or expired",
        ],
    ]);
});

test("rotate to a given value, keeping the rotated-out version live through its overlap and earlier history out of the live versions", async () => {
    const fixture = await TestVault.open();
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    const { key, request } = await fixture.createSecret(caller);

    // add a second version as history, then rotate to a third with a minute's overlap
    const second = await client.version.create({
        ...request,
        requestId: RequestId.create(fixture.environment),
    });
    await fixture.point(caller, key, second.number);
    const rotatedAt = fixture.clock.now();
    const value = { encoding: "text" as const, value: "rotated" };
    await client.secret.rotate({
        ...key,
        requestId: RequestId.create(fixture.environment),
        value,
        expiresIn: { minutes: 1 },
    });
    const during = await client.secret.versions(key);

    // drop the rotated-out version once its overlap ends
    await fixture.space.simulation.run(rotatedAt + Duration.milliseconds({ minutes: 1 }));
    const after = await client.secret.versions(key);

    // refuse rotating without a value a secret written as given, which has no algorithm
    const refused = await refusal(
        client.secret.rotate({ ...key, requestId: RequestId.create(fixture.environment) }),
    );

    expect({ during, after, refused }).toEqual({
        during: [
            { version: 3, value },
            { version: 2, value: CREDENTIAL },
        ],
        after: [{ version: 3, value }],
        refused: [
            "BAD_REQUEST",
            "VALUE_REQUIRED",
            "value is required: a secret without an algorithm rotates to a given value",
        ],
    });
});

test("rotate a generated secret to a new value of its algorithm, a capture of it using the version retiring beside it", async () => {
    // install the application with a key the space generates, then rotate it as the owner
    const fixture = await TestVault.open({ secrets: [KEY] });
    const { database } = fixture.served;
    const row = single(await database.select().from(secret.table));
    const key = { scope: ids.space, id: row.id };
    const client = (await fixture.elevated()).client(vaultPackage);
    const [first] = await client.secret.versions(key);
    await client.secret.rotate({ ...key, requestId: RequestId.create(fixture.environment) });
    const rotated = await client.secret.versions(key);

    // read what a capture of the new version uses
    const versions = await database
        .select()
        .from(secretVersion.table)
        .orderBy(secretVersion.table.number);
    const uses = await secretBindable.uses(
        { scope: ids.space, target: row.id, version: 2 },
        Snapshot.live(database),
    );

    // generate signing keys, keeping the first live for a day beside the new one
    const texts = rotated.map((reading) => reading.value.value);
    const Jwk = schema.looseObject({ alg: schema.string() });
    expect({
        algorithm: single(await database.select().from(secret.table)).algorithm,
        versions: rotated.map((reading) => reading.version),
        algorithms: texts.map((text) => Jwk.parse(JSON.parse(text)).alg),
        kept: rotated[1]?.value.value === first?.value.value,
        isNew: texts[0] !== texts[1],
        uses,
    }).toEqual({
        algorithm: "ES256",
        versions: [2, 1],
        algorithms: ["ES256", "ES256"],
        kept: true,
        isNew: true,
        uses: [
            secret.reference(ids.space, row.id),
            ...[versions[1], versions[0]].map((version) =>
                secretVersion.reference(ids.space, String(version?.id)),
            ),
        ],
    });
});

test("keep a secret's name in its vault until its deletion, then refuse restoring it while another secret has the name", async () => {
    const fixture = await TestVault.open();
    const client = fixture.owner.client(vaultPackage);
    const vaultId = await fixture.vault();
    const create = () =>
        client.secret.create({
            scope: ids.space,
            parentId: vaultId,
            name: "token",
            requestId: RequestId.create(fixture.environment),
        });

    // refuse the name while the first secret is live
    const first = await create();
    const taken = await refusal(create());

    // delete the first secret, which frees its name for a second one
    const key = { scope: ids.space, id: first.id };
    await client.secret.delete({
        ...key,
        revision: first.revision,
        requestId: RequestId.create(fixture.environment),
    });
    const second = await create();
    const restored = await refusal(
        client.secret.restore({ ...key, requestId: RequestId.create(fixture.environment) }),
    );

    // expect the name taken twice, and given to the second secret
    const conflict = [
        "CONFLICT",
        "OBJECT_EXISTS",
        "'secret' with the same 'parent and name' already exists",
    ];
    expect([taken, second.name, restored]).toEqual([conflict, "token", conflict]);
});

test("purge a deleted secret at once or when its window ends, destroying its values and keeping its records", async () => {
    const fixture = await TestVault.open();
    const client = fixture.owner.client(vaultPackage);
    const { database } = fixture.served;
    const { key, version: written } = await fixture.createSecret(fixture.owner);

    // delete it at a paused time
    const deletedAt = fixture.clock.now();
    await client.secret.delete({
        ...key,
        revision: (await client.secret.get(key)).revision,
        requestId: RequestId.create(fixture.environment),
    });
    const deleted = await client.secret.get(key);

    // purge another deleted secret within its window
    const other = await client.secret.create({
        scope: ids.space,
        parentId: await fixture.vault(),
        name: "other",
        requestId: RequestId.create(fixture.environment),
    });
    const otherKey = { scope: ids.space, id: other.id };
    await client.secret.delete({
        ...otherKey,
        revision: other.revision,
        requestId: RequestId.create(fixture.environment),
    });
    const otherDeleted = await client.secret.get(otherKey);
    await client.secret.purge({ ...otherKey, requestId: RequestId.create(fixture.environment) });
    const otherPurged = await client.secret.get(otherKey);

    // destroy every value once the window ends, keeping the secret and its versions as records
    const version = single(
        await database
            .select()
            .from(secretVersion.table)
            .where(eq(secretVersion.table.id, written.id)),
    );
    const windowEnd = deletedAt + RECOVERY_MILLISECONDS;
    await fixture.space.simulation.jump(windowEnd);

    // sign the owner in again, as their sign-in lapsed within the window
    const owner = await fixture.space.universe.signIn(fixture.space.owner.email);
    const later = fixture.space.person(fixture.served, owner).client(vaultPackage);
    const kept = {
        secret: await later.secret.get(key),
        version: single(
            await database
                .select()
                .from(secretVersion.table)
                .where(eq(secretVersion.table.id, written.id)),
        ),
    };

    // read the code serving secrets, which destroys them
    const serving = fixture.served.store.as(secret).subject;
    expect({ otherPurged, kept }).toEqual({
        otherPurged: { ...otherDeleted, purgedAt: deletedAt, revision: otherDeleted.revision + 1 },
        kept: {
            secret: {
                ...deleted,
                purgedAt: windowEnd,
                revision: deleted.revision + 1,
                updatedAt: windowEnd,
                ...ObjectReference.columns("updatedBy", serving),
            },
            version: {
                ...version,
                state: "destroyed",
                content: null,
                revision: version.revision + 1,
                updatedAt: windowEnd,
                ...ObjectReference.columns("updatedBy", serving),
            },
        },
    });
});

test("refuse deleting a secret its installation's declaration manages", async () => {
    // install the application with a key generated in its vault, bound to the installation
    const fixture = await TestVault.open({ secrets: [KEY] });
    const client = fixture.owner.client(vaultPackage);
    const bound = single(await fixture.served.database.select().from(secret.table));
    const current = await client.secret.get({ scope: ids.space, id: bound.id });

    // refuse the deletion, which the installation's declaration alone retires
    const refused = await refusal(
        client.secret.delete({
            scope: ids.space,
            id: bound.id,
            revision: current.revision,
            requestId: RequestId.create(fixture.environment),
        }),
    );
    expect(refused).toEqual([
        "PRECONDITION_FAILED",
        "DECLARATION_BLOCKED",
        "cannot change 'secret': it is managed by its declaration, so detach it first",
    ]);
});

test("refuse reading a version whose stored value was copied from another version", async () => {
    // write two versions of a secret, the second current
    const fixture = await TestVault.open();
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    const { database } = fixture.served;
    const { key, version: written } = await fixture.createSecret(caller);
    const second = await client.version.create({
        scope: ids.space,
        parentId: key.id,
        requestId: RequestId.create(fixture.environment),
        value: { encoding: "text", value: "rotated" },
    });
    await fixture.point(caller, key, second.number);

    // copy the first version's stored value into the second, as a write to the database could
    const stored = single(
        await database
            .select({ content: secretVersion.table.content })
            .from(secretVersion.table)
            .where(eq(secretVersion.table.id, written.id)),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(secretVersion.table)
                .set({ content: stored.content })
                .where(eq(secretVersion.table.id, second.id)),
    );

    // refuse the copied value as a hidden server fault, and keep reading the first version's own
    const copied = single(
        await database
            .select()
            .from(secretVersion.table)
            .where(eq(secretVersion.table.id, second.id)),
    );
    expect([
        await refusal(
            Promise.resolve().then(() =>
                SecretVersion.requireReadable(copied, fixture.clock.now()),
            ),
        ),
        await refusal(client.secret.read(key)),
        await client.secret.read({ ...key, version: 1 }),
    ]).toEqual([
        [
            "INTERNAL_SERVER_ERROR",
            "VERSION_BROKEN",
            `secret version '${second.id}' contains the value of another version`,
        ],
        ["INTERNAL_SERVER_ERROR", "INTERNAL_SERVER_ERROR", "internal server error"],
        FIRST,
    ]);
});

test("deny plaintext while the space is suspended, keeping its metadata readable", async () => {
    const fixture = await TestVault.open();
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    const { key } = await fixture.createSecret(caller);
    const metadata = await client.secret.get(key);

    // suspend the space at its primary, then resume it
    await fixture.space.change("suspend");
    const suspended = [await refusal(client.secret.read(key)), await client.secret.get(key)];
    await fixture.space.change("resume");
    expect([suspended, await client.secret.read(key)]).toEqual([
        [
            [
                "FORBIDDEN",
                "SCOPE_DENIED",
                "scope 'space-01996ab0-0000-7000-8000-000000000002' is forbidden: it is suspended",
            ],
            metadata,
        ],
        FIRST,
    ]);
});

test("deny plaintext to a member who may not reveal, keeping metadata readable", async () => {
    const fixture = await TestVault.open();
    const { key } = await fixture.createSecret(fixture.owner);
    const member = await fixture.elevated(await fixture.member(CONCEALING));
    const client = member.client(vaultPackage);

    // read the metadata, and refuse the value
    expect([await client.secret.get(key), await refusal(client.secret.read(key))]).toEqual([
        await fixture.owner.client(vaultPackage).secret.get(key),
        ["FORBIDDEN", "PERMISSION_DENIED", "permission 'read' is forbidden: the caller lacks it"],
    ]);
});

test("reveal a missing version only to readers of the secret", async () => {
    const fixture = await TestVault.open();
    const { key } = await fixture.createSecret(fixture.owner);
    const missing = { ...key, version: 2 };
    const person = await fixture.member(CONCEALING);
    const member = await fixture.elevated(person);

    // refuse a caller reading neither the secret nor the version, and tell a reader the version is missing
    const concealed = await refusal(member.client(vaultPackage).secret.read(missing));
    await fixture.allow(person, VAULT_PERMISSIONS);
    const told = await refusal(member.client(vaultPackage).secret.read(missing));
    expect([concealed, told]).toEqual([
        ["FORBIDDEN", "PERMISSION_DENIED", "permission 'read' is forbidden: the caller lacks it"],
        ["NOT_FOUND", "VERSION_NOT_FOUND", "secret version not found"],
    ]);
});

test("challenge a person's reveal for a step-up, and reveal once they authenticated again", async () => {
    const fixture = await TestVault.open();
    const { key } = await fixture.createSecret(fixture.owner);

    // refuse revealing on a sign-in without a fresh second factor, and reveal once stepped up
    const refused = await refusal(fixture.owner.client(vaultPackage).secret.read(key));
    const revealed = await (await fixture.elevated()).client(vaultPackage).secret.read(key);
    expect([refused, revealed]).toEqual([
        [
            "UNAUTHORIZED",
            "ASSURANCE_UNAUTHENTICATED",
            "authentication required: authentication assurance is too low",
        ],
        FIRST,
    ]);
});

test("generate a declared key in the vault its installation binds, and read what a capture of each version uses", async () => {
    // install the application declaring its credentials vault and a key generated in it
    const fixture = await TestVault.open({ secrets: [KEY] });
    const { database } = fixture.served;

    // read the generated key, its binding and what a capture of each version uses
    const row = single(await database.select().from(secret.table));
    const version = single(await database.select().from(secretVersion.table));
    const value = SecretVersion.requireReadable(version, fixture.clock.now());
    const jwk = schema
        .looseObject({ kty: schema.string(), crv: schema.string(), alg: schema.string() })
        .parse(JSON.parse(value.value));
    const bound = (await targetedBindings(database))
        .filter((entry) => entry.target === row.id)
        .map((entry) => ({ installationId: entry.installationId, name: entry.name }));
    const snapshot = Snapshot.live(database);
    const uses = (captured: number | null) =>
        secretBindable.uses({ scope: ids.space, target: row.id, version: captured }, snapshot);
    expect({
        secret: [row.parentId, row.name, row.currentVersion],
        key: [jwk.kty, jwk.crv, jwk.alg],
        bound,
        versions: [secretBindable.version(row, null), secretBindable.version(row, 3)],
        uses: [await uses(null), await uses(1)],
    }).toEqual({
        secret: [await fixture.vault(), "key", 1],
        key: ["EC", "P-256", "ES256"],
        bound: [{ installationId: fixture.installation, name: "key" }],
        versions: [1, 3],
        uses: [
            [secret.reference(ids.space, row.id)],
            [secret.reference(ids.space, row.id), secretVersion.reference(ids.space, version.id)],
        ],
    });
});

test("read a bound secret in process as its installation, and refuse another installation's secret", async () => {
    // install the application with a generated key, and another application with its own
    const fixture = await TestVault.open({ secrets: [KEY] });
    const other = CONSUMER;
    const otherKey = defineSecret(
        { name: "key", generated: { vault: CREDENTIALS, algorithm: "ES256" } },
        { package: other },
    );
    const built = await TestVault.build(
        other,
        [defineVault({ name: CREDENTIALS }, { package: other })],
        [otherKey],
        fixture.environment.crypto,
    );
    const otherInstallation = await fixture.space.install(fixture.served, built);

    // connect the application to its own key and to the other application's, as its server in process
    const { database } = fixture.served;
    const store = Promise.resolve(fixture.served.store);
    const snapshot = Snapshot.live(database);
    const rows = await snapshot
        .read(rollout.table)
        .findMany({ where: { installationId: fixture.installation } });
    const row = single(await Rollout.read(snapshot, rows));
    const keyOf = async (installationId: Identifier<"installation">) =>
        single(
            (await Binding.targeted(Snapshot.live(database), ids.space, installationId)).filter(
                (entry) => entry.name === "key",
            ),
        );
    const strayBinding = await keyOf(otherInstallation);
    const strayId = strayBinding.target;
    const ownBinding = await keyOf(fixture.installation);
    const own = await secretBindable.connect(store, row, ownBinding);
    const stray = await secretBindable.connect(store, row, strayBinding);

    // read its own key's stored value, and find no secret of the other's, which it may not see
    const read = await own.read();
    const stored = single(
        await database
            .select()
            .from(secretVersion.table)
            .where(eq(secretVersion.table.parentId, secret.identifier(ownBinding.target))),
    );
    expect({ read, refused: await refusal(stray.read()) }).toEqual({
        read: { version: 1, value: SecretVersion.requireReadable(stored, fixture.clock.now()) },
        refused: ["NOT_FOUND", "NOT_FOUND", `secret '${strayId}' not found`],
    });
});

test("fail installing a key generated in a vault the installation declares nowhere", async () => {
    // install the application declaring a key generated in a vault it lacks, and read the installation's failures
    const lost = defineSecret(
        { name: "key", generated: { vault: "mail", algorithm: "ES256" } },
        { package: APPLICATION },
    );
    const refused = await TestSpace.refusals(TestVault.open({ secrets: [lost] }));
    expect(refused).toEqual([
        ["BAD_REQUEST", "DECLARATION_INVALID", "vault 'mail' is invalid: it is unknown"],
    ]);
});

test("bind a secret the stack declares at a fixed version, and trash it once the stack drops it", async () => {
    // declare a vault and a secret in it through the space's stack, with two versions
    const fixture = await TestVault.open({ vaults: [] });
    await fixture.space.apply(fixture.served, [vaultPackage], {
        vaults: { shared: SHARED.provision({ retention: "delete" }) },
        secrets: { github: { parent: "shared", name: "github" } },
    });
    const declared = single(await fixture.served.database.select().from(secret.table));
    const client = fixture.owner.client(vaultPackage);
    for (const value of ["first", "second"]) {
        await client.version.create({
            scope: ids.space,
            parentId: declared.id,
            requestId: RequestId.create(fixture.environment),
            value: { encoding: "text", value },
        });
    }

    // install the consumer bound to the first version of the stack's secret
    const consumer = await fixture.space.install(
        fixture.served,
        await TestVault.build(
            CONSUMER,
            [],
            [defineSecret({ name: "token" }, { package: CONSUMER })],
            fixture.environment.crypto,
        ),
        {
            bindings: {
                [CONSUMER.id]: {
                    token: { target: { type: "secret", name: "github" }, version: 1 },
                },
            },
        },
    );
    const bound = (await targetedBindings(fixture.served.database))
        .filter((entry) => entry.name === "token")
        .map((entry) => ({
            installationId: entry.installationId,
            target: entry.target,
            version: entry.version,
        }));

    // drop the consumer and the secret from the stack at a paused time, trashing the secret
    const before = await client.secret.get({ scope: ids.space, id: declared.id });
    const droppedAt = fixture.clock.now();
    await fixture.space.apply(fixture.served, [vaultPackage], { installations: {}, secrets: {} });
    expect({
        bound,
        trashed: await client.secret.get({ scope: ids.space, id: declared.id }),
    }).toEqual({
        bound: [{ installationId: consumer, target: declared.id, version: 1 }],
        trashed: {
            ...before,
            deletedAt: droppedAt,
            ...ObjectReference.columns("deletedBy", SPACE),
            revision: before.revision + 1,
            updatedAt: droppedAt,
        },
    });
});

test("point a secret the stack declares at a version as the space's owner, keeping it across the stack's next apply", async () => {
    // declare a vault and a secret in it through the space's stack, and write a version
    const fixture = await TestVault.open({ vaults: [] });
    const stack = {
        vaults: { shared: SHARED.provision({ retention: "delete" }) },
        secrets: { github: { parent: "shared", name: "github" } },
    };
    await fixture.space.apply(fixture.served, [vaultPackage], stack);
    const declared = single(await fixture.served.database.select().from(secret.table));
    const caller = await fixture.elevated();
    const client = caller.client(vaultPackage);
    const version = await client.version.create({
        scope: ids.space,
        parentId: declared.id,
        requestId: RequestId.create(fixture.environment),
        value: { encoding: "text", value: "first" },
    });

    // point the current alias at it, and apply the stack again
    const key = { scope: ids.space, id: declared.id };
    await fixture.point(caller, key, version.number);
    await fixture.space.apply(fixture.served, [vaultPackage], stack);
    const kept = await client.secret.get(key);
    expect({ current: kept.currentVersion, name: kept.name }).toEqual({
        current: version.number,
        name: "github",
    });
});

/** Read the bindings of the space's targeted revisions, with their installations. */
async function targetedBindings(database: DatabaseConnection) {
    const snapshot = Snapshot.live(database);
    const rollouts = await snapshot.read(rollout.table).findMany({ where: { scope: ids.space } });

    return (
        await Promise.all(
            rollouts.map(async (row) =>
                (await Binding.targeted(snapshot, ids.space, row.installationId)).map((entry) => ({
                    ...entry,
                    installationId: row.installationId,
                })),
            ),
        )
    ).flat();
}
