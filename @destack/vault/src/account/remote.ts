import type { Vault } from "@destack/account/server";
import type { Subject } from "@destack/sync";
import { isServiceError, ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import type { SecretClient } from "../object/index.ts";

/** A vault reached over a space's vault service. */
export class RemoteVault implements Vault {
    /** Connect to the vault serving a space, as a principal or as the account service. */
    readonly connect: (spaceId: string, subject?: Subject) => Promise<SecretClient>;

    /** Reach vaults through the given connections. */
    constructor(connect: RemoteVault["connect"]) {
        this.connect = connect;
    }

    /** Create the secret with the value as its first version, once. */
    async write(secret: Parameters<Vault["write"]>[0]): Promise<void> {
        // create the secret under its identifier, else read the one an earlier write created
        const client = await this.connect(secret.spaceId, secret.subject);
        const { spaceId, id } = secret;
        const existing =
            (await absent(
                client.secret.create({
                    spaceId,
                    parentId: secret.vaultId,
                    id,
                    name: secret.name,
                    requestId: RequestId.create(),
                }),
                "CONFLICT",
            )) ?? (await client.secret.get({ spaceId, id }));

        // keep the value as the first version, unless an earlier write did
        if (existing.currentVersion === null) {
            await client.version.create({
                spaceId,
                parentId: id,
                value: { encoding: "text", value: secret.value },
                requestId: RequestId.create(),
            });
        }
    }

    /** Read the secret's current text value. */
    async read(secret: Parameters<Vault["read"]>[0]): Promise<string> {
        // read the current version
        const client = await this.connect(secret.spaceId, secret.subject);
        const { value } = await client.secret.read({
            spaceId: secret.spaceId,
            id: secret.secretId,
        });
        if (value.encoding !== "text") {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `secret ${secret.secretId} has no text credential`,
            });
        }

        return value.value;
    }

    /** Delete the secret and purge its values, doing nothing for a secret already gone. */
    async destroy(secret: Parameters<Vault["destroy"]>[0]): Promise<void> {
        // read the secret and skip one already missing or purged
        const client = await this.connect(secret.spaceId, secret.subject);
        const key = { spaceId: secret.spaceId, id: secret.secretId };
        const existing = await absent(client.secret.get(key), "NOT_FOUND");
        if (existing === undefined || existing.purgedAt !== null) {
            return;
        }

        // move it to the trash unless an earlier destroy did, then purge its values
        if (existing.deletionRequestedAt === null) {
            await client.secret.delete({
                ...key,
                revision: existing.revision,
                requestId: RequestId.create(),
            });
        }
        await client.secret.purge({ ...key, requestId: RequestId.create() });
    }
}

/** Await a vault call, answering undefined for the one expected failure. */
async function absent<Value>(
    pending: Promise<Value>,
    code: "CONFLICT" | "NOT_FOUND",
): Promise<Value | undefined> {
    try {
        return await pending;
    } catch (error) {
        // answer the expected failure, and rethrow every other one
        if (isServiceError(error) && error.code === code) {
            return undefined;
        }
        throw error;
    }
}
