import { AccessContext, Caller } from "@destack/access";
import { type Table } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { RequestId } from "@destack/runtime/request";
import { expect, refusal, single, test } from "@destack/test";
import { secret, secretVersion, vault } from "../object/index.ts";
import { VaultFixture } from "../test/index.ts";

test.each(TEST_DIALECTS)(
    "refuse finishing a vault's deletion while it keeps a secret, and finish it once the secret is purged on %s",
    async (dialect) => {
        // create a live secret in the vault
        await using fixture = await VaultFixture.open(dialect);
        const { client, database } = fixture;
        const { key } = await fixture.createSecret();
        const machine = fixture.objects();
        const send = async (name: "delete" | "finalize") => {
            const stored = single(await database.select().from(vault.table));
            await machine.mutate(AccessContext.of(Caller.of(machine.principal), Date.now()), {
                id: RequestId.create(Date.now()),
                calls: [vault.call(name, vault.target(stored))],
            });
        };

        // request the vault's deletion, and refuse finishing it while the vault keeps the secret
        await send("delete");
        const refused = await refusal(send("finalize"));

        // finish the deletion with the vault's records once the secret is purged
        const row = await client.secret.get(key);
        await client.secret.delete({
            ...key,
            revision: row.revision,
            requestId: RequestId.create(Date.now()),
        });
        await client.secret.purge({ ...key, requestId: RequestId.create(Date.now()) });
        await send("finalize");
        const remaining = await Promise.all(
            [vault.table, secret.table, secretVersion.table].map((table: Table) =>
                database.select().from(table),
            ),
        );
        expect({ refused, remaining }).toEqual({
            refused: ["BROKEN_REFERENCE", "the change would leave a reference to a missing record"],
            remaining: [[], [], []],
        });
    },
);
