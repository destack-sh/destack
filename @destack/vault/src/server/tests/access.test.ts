import { TEST_DIALECTS } from "@destack/db/test";
import { suspendCopy } from "@destack/access/test";
import { ServiceError } from "@destack/service/error";
import { expect, test } from "@destack/test";
import { VaultFixture } from "./fixture.ts";

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
        await suspendCopy(database, spaceId, Date.now());
        await expect(client.secret.read(key)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "permission denied: open" }),
        );
        expect(await client.secret.get(key)).toEqual(metadata);

        // read the same secret again once resumed
        await suspendCopy(database, spaceId, null);
        expect(await client.secret.read(key)).toEqual(CREDENTIAL);
    },
);

test.each(TEST_DIALECTS)("deny plaintext apart from metadata on %s", async (dialect) => {
    await using fixture = await VaultFixture.open(dialect);
    const { client } = fixture;
    const { key, written } = await fixture.createSecret();

    // keep metadata readable once the role no longer reads plaintext
    await fixture.allowRead(false);
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
        await fixture.allowRead(false);
        await expect(client.secret.read(missing)).rejects.toEqual(
            new ServiceError("FORBIDDEN", { defined: true, message: "permission denied: read" }),
        );

        // tell a reader of the secret that the version is missing
        await fixture.allowRead(true);
        await expect(client.secret.read(missing)).rejects.toEqual(
            new ServiceError("NOT_FOUND", { defined: true, message: "secret version not found" }),
        );
    },
);
