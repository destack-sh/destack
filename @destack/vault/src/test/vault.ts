import type { DatabaseConnection } from "@destack/db";
import { type Identifier, schema } from "@destack/schema";
import type { VaultKeystore } from "../key/index.ts";
import { vaultKey } from "../object/index.ts";

/** The vault rows a test writes directly. */
export const TestVault = {
    /** Start a vault's key at its first version under a keystore. */
    async startKey(
        database: DatabaseConnection,
        keystore: VaultKeystore,
        created: { readonly id: Identifier<"vault">; readonly scope: string },
        now: number = Date.now(),
    ): Promise<void> {
        const id = schema.identifier("vault-key").parse(vaultKey.generateId());
        await database.insert(vaultKey.table).values({
            id,
            scope: created.scope,
            parentId: created.id,
            number: 1,
            ciphertext: await keystore.generate(created.id, id),
            createdAt: now,
            updatedAt: now,
        });
    },
};
