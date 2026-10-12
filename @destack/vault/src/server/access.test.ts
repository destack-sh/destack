import { AccessFixture } from "@destack/access/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { expect, refusal, test } from "@destack/test";
import { v7 } from "uuid";
import { VaultFixture } from "../test/index.ts";

/** The value of the fixture's first version. */
const CREDENTIAL = { version: 1, value: { encoding: "text", value: "credential" } };

test.each(TEST_DIALECTS)(
    "deny plaintext while keeping suspended space administration on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { database, client, spaceId } = fixture;
        const { key } = await fixture.createSecret();
        const metadata = await client.secret.get(key);

        // suspend the space without deleting its resources or granting new permissions
        const copies = new AccessFixture(database);
        await copies.suspendCopy(spaceId, Date.now());
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "permission denied: open" }),
        );
        expect(await client.secret.get(key)).toEqual(metadata);

        // read the same secret again once resumed
        await copies.suspendCopy(spaceId, null);
        expect(await client.secret.read(key)).toEqual(CREDENTIAL);
    },
);

test.each(TEST_DIALECTS)("deny plaintext apart from metadata on %s", async (dialect) => {
    await using fixture = await VaultFixture.open(dialect);
    const { client } = fixture;
    const { key, written } = await fixture.createSecret();

    // keep metadata readable once the role no longer reads plaintext
    await fixture.withdrawRead();
    expect((await client.secret.get(key)).currentVersion).toBe(written.number);
    await expect(client.secret.read(key)).rejects.toEqual(
        new ServiceError("FORBIDDEN", { defined: true, message: "permission denied: read" }),
    );
});

test.each(TEST_DIALECTS)(
    "reveal a missing version only to readers of the secret on %s",
    async (dialect) => {
        await using fixture = await VaultFixture.open(dialect);
        const { client } = fixture;
        const { key } = await fixture.createSecret();
        const missing = { ...key, version: 2 };

        // refuse a caller who reads neither the secret nor the version before looking the version up
        await fixture.withdrawRead();
        await expect(client.secret.read(missing)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "permission denied: read" }),
        );

        // tell a reader of the secret that the version is missing
        await fixture.grantRead();
        await expect(client.secret.read(missing)).rejects.toEqual(
            new ServiceError("NOT_FOUND", { defined: true, message: "secret version not found" }),
        );
    },
);

test.each(TEST_DIALECTS)(
    "read a vault's secret by name, never a secret of the same name in another vault, on %s",
    async (dialect) => {
        // keep a secret of the same name in the fixture's vault and another one
        await using fixture = await VaultFixture.open(dialect);
        const { client, spaceId, vaultId } = fixture;
        await fixture.createSecret();
        const otherId = schema.identifier("vault").parse(`vault-${v7()}`);
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
        await fixture.withdrawRead();
        const refused = await refusal(read(vaultId, "first"));
        expect({ own, another, missing, refused }).toEqual({
            own: CREDENTIAL,
            another: { version: 1, value: { encoding: "text", value: "other credential" } },
            missing: ["NOT_FOUND", "vault has no secret second"],
            refused: ["FORBIDDEN", "permission denied: read"],
        });
    },
);
