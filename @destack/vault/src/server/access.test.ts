import { Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { Identifier } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { expect, refusal, test } from "@destack/test";
import { vault } from "../object/index.ts";
import { VaultFixture } from "../test/index.ts";

/** The value of the fixture's first version. */
const CREDENTIAL = { version: 1, value: { encoding: "text", value: "credential" } };

test.each(TEST_DIALECTS)(
    "deny plaintext while keeping suspended space administration on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        fixture.stepUp();
        const { database, client, spaceId } = fixture;
        const { key } = await fixture.createSecret();
        const metadata = await client.secret.get(key);

        // suspend the space without deleting its resources or granting new permissions
        const access = new AccessFixture(database);
        await access.suspendReplica(spaceId, Date.now());
        await expect(client.secret.read(key)).rejects.toEqual(
            new CallError("FORBIDDEN", { defined: true, message: "permission denied: read" }),
        );
        expect(await client.secret.get(key)).toEqual(metadata);

        // read the same secret again once resumed
        await access.suspendReplica(spaceId, null);
        expect(await client.secret.read(key)).toEqual(CREDENTIAL);
    },
);

test.each(TEST_DIALECTS)("deny plaintext apart from metadata on %s", async (dialect) => {
    await using fixture = await VaultFixture.open(dialect);
    fixture.stepUp();
    const { client } = fixture;
    const { key, written } = await fixture.createSecret();

    // keep metadata readable once the role no longer reads plaintext
    await fixture.withdrawReveal();
    expect((await client.secret.get(key)).currentVersion).toBe(written.number);
    await expect(client.secret.read(key)).rejects.toEqual(
        new CallError("FORBIDDEN", { defined: true, message: "permission denied: read" }),
    );
});

test.each(TEST_DIALECTS)(
    "reveal a missing version only to readers of the secret on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        fixture.stepUp();
        const { client } = fixture;
        const { key } = await fixture.createSecret();
        const missing = { ...key, version: 2 };

        // refuse a caller who reads neither the secret nor the version before looking the version up
        await fixture.withdrawReveal();
        await expect(client.secret.read(missing)).rejects.toEqual(
            new CallError("FORBIDDEN", { defined: true, message: "permission denied: read" }),
        );

        // tell a reader of the secret that the version is missing
        await fixture.grantReveal();
        await expect(client.secret.read(missing)).rejects.toEqual(
            new CallError("NOT_FOUND", { defined: true, message: "secret version not found" }),
        );
    },
);

test.each(TEST_DIALECTS)(
    "read a vault's secret by name, never a secret of the same name in another vault, on %s",
    async (dialect) => {
        // keep a secret of the same name in the fixture's vault and another one
        await using fixture = await VaultFixture.open(dialect);
        fixture.stepUp();
        const { client, spaceId, vaultId } = fixture;
        await fixture.createSecret();
        const otherId = Identifier.create("vault");
        await fixture.registerVault(otherId, "other");
        const other = await client.secret.create({
            scope: spaceId,
            parentId: otherId,
            name: "first",
            requestId: RequestId.create(Date.now()),
        });
        await client.version.create({
            scope: spaceId,
            parentId: other.id,
            requestId: RequestId.create(Date.now()),
            value: { encoding: "text", value: "other credential" },
        });
        const read = (id: typeof vaultId, name: string) =>
            client.vault.read({ scope: spaceId, id, name, version: 1 });

        // read each vault's own secret, and nothing the vault lacks
        const own = await read(vaultId, "first");
        const another = await read(otherId, "first");
        const missing = await refusal(read(vaultId, "second"));

        // refuse a caller who reads neither the secret nor its version
        await fixture.withdrawReveal();
        const refused = await refusal(read(vaultId, "first"));
        expect({ own, another, missing, refused }).toEqual({
            own: CREDENTIAL,
            another: { version: 1, value: { encoding: "text", value: "other credential" } },
            missing: ["NOT_FOUND", "vault has no secret second"],
            refused: ["NOT_FOUND", "vault has no secret first"],
        });
    },
);

test.each(TEST_DIALECTS)(
    "challenge a person's reveal for a step-up, and reveal once they authenticated again, on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { client } = fixture;
        const { key } = await fixture.createSecret();

        // refuse revealing on a sign-in without a fresh second factor
        const refused = await refusal(client.secret.read(key));

        // reveal once the member stepped up
        fixture.stepUp();
        expect([refused, await client.secret.read(key)]).toEqual([
            [
                "INSUFFICIENT_AUTHENTICATION",
                "authentication assurance is too low: authenticate again",
            ],
            CREDENTIAL,
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "let an installation bound to a vault read its secrets by name without a step-up, and conceal the vault from another, on %s",
    async (dialect) => {
        // bind one installation to the fixture's vault as the space, and leave another unbound
        await using fixture = await VaultFixture.open(dialect);
        const { spaceId, vaultId } = fixture;
        await fixture.createSecret();
        const store = fixture.objects();
        const bound = principal.installation.reference(spaceId, Identifier.create("installation"));
        const unbound = principal.installation.reference(
            spaceId,
            Identifier.create("installation"),
        );
        await store.mutate(Caller.of(store.principal, Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [vault.call("bind", { ...vault.in(spaceId), consumer: bound, ids: [vaultId] })],
        });

        // read the secret by name as each installation
        const read = (installation: typeof bound) =>
            store.query(
                Caller.of(installation, Date.now()),
                vault.call("read", {
                    ...vault.target({ scope: spaceId, id: vaultId }),
                    name: "first",
                }),
            );
        expect([await read(bound), await refusal(read(unbound))]).toEqual([
            CREDENTIAL,
            ["NOT_FOUND", `no vault ${vaultId}`],
        ]);
    },
);
