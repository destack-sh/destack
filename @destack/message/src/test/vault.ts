import { AccessContext, Caller, principal } from "@destack/access";
import { and, eq } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { Identifier, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { type ObjectReference, Scope, type Subject } from "@destack/sync";
import { VAULT_PROVIDER } from "@destack/vault/declare";
import { secret, vault } from "@destack/vault/object";

/** The name of the vault with a fixture space's secrets. */
const VAULT_NAME = "secrets";

/** Store a text as the vault secret a reference points at, with its consumer. */
export async function storeSecret(
    store: ObjectStore,
    kept: ObjectReference,
    value: string,
    consumer: Subject,
): Promise<void> {
    // create the space's vault as its stack provisions it, once
    const now = Date.now();
    const scope = schema.identifier("space").parse(kept.scope);
    const [existing] = await store.database
        .select()
        .from(vault.table)
        .where(and(eq(vault.table.scope, scope), eq(vault.table.name, VAULT_NAME)));
    const vaultId = existing?.id ?? Identifier.create("vault");
    if (existing === undefined) {
        await store.database.insert(vault.table).values({
            id: vaultId,
            scope,
            name: VAULT_NAME,
            definitionPackageId: vault.package.id,
            definitionVersion: vault.package.version,
            definitionName: VAULT_NAME,
            spec: {},
            provider: VAULT_PROVIDER,
            reference: vault.package.name,
            createdAt: now,
            updatedAt: now,
        });
    }

    // put the value as the space, as the secret's one live version for its consumer
    await store.mutate(
        AccessContext.of(
            Caller.of(principal.space.reference(Scope.universe.id, scope)),
            Date.now(),
        ),
        {
            id: RequestId.create(now),
            calls: [
                vault.call("put", {
                    ...vault.target({ scope, id: vaultId }),
                    secret: secret.identifier(kept.id),
                    name: kept.id,
                    value: { encoding: "text", value },
                    consumer,
                }),
            ],
        },
    );
}
