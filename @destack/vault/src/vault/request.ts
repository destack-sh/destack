import { and, eq } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { SecretEnvelope } from "../encryption/index.ts";
import type { Secret } from "../secret/index.ts";
import { vaultRequest } from "../stack/index.ts";
import { requestRewrap } from "../audit/index.ts";
import { authorizeVault } from "./access.ts";
import type { Vault } from "./vault.ts";
import type { VaultContext } from "./context.ts";

/** Rewrap a bounded batch of retry digests before retiring an old root key. */
export async function rewrapRequests(
    vault: Vault,
    input: { spaceId: Secret["spaceId"]; keyId: string; limit: number },
    context: VaultContext,
): Promise<number> {
    // bound each rewrapping transaction
    const { spaceId, keyId, limit } = input;
    if (!Number.isInteger(limit) || limit < 1 || limit > 1000) {
        throw new ServiceError("BAD_REQUEST");
    }

    return vault.database.transaction(
        async (transaction) => {
            // authorize the entire space before inspecting protected retry records
            await authorizeVault(context, { operation: "request.rewrap", spaceId }, transaction);

            // select only records still protected by the retiring key
            const rows = await transaction
                .select()
                .from(vaultRequest)
                .where(and(eq(vaultRequest.scope, spaceId), eq(vaultRequest.keyId, keyId)))
                .limit(limit);

            // preserve request identity while replacing root-key protection
            for (const row of rows) {
                const identity = vault.requestContext(row);
                const envelope = await vault.encryption.rewrap(
                    SecretEnvelope.parse(row.digest),
                    identity,
                );
                if (envelope.keyId === keyId) {
                    throw new ServiceError("CONFLICT", {
                        message: "replacement root key must differ",
                    });
                }

                // commit each replacement with its audit event
                await transaction
                    .update(vaultRequest)
                    .set({ keyId: envelope.keyId, digest: envelope })
                    .where(
                        and(
                            eq(vaultRequest.caller, row.caller),
                            eq(vaultRequest.scope, row.scope),
                            eq(vaultRequest.procedure, row.procedure),
                            eq(vaultRequest.requestId, row.requestId),
                            eq(vaultRequest.keyId, keyId),
                        ),
                    );
                await context.audit.record(transaction, requestRewrap, {
                    targets: { space: { type: "space", id: spaceId } },
                    details: { previousKeyId: keyId, keyId: envelope.keyId, count: 1 },
                    outcome: "success",
                });
            }

            return rows.length;
        },
        { isolationLevel: "serializable" },
    );
}
