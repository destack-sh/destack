import type { Vault } from "@destack/account/server";
import type { ObjectReference, Subject } from "@destack/sync";
import { schema } from "@destack/schema";
import { isServiceError, ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { secret, type SecretClient, vault } from "../object/index.ts";

/** A vault reached over a space's vault service. */
export class RemoteVault implements Vault {
    /** Connect to the vault serving a space, as a principal or as the account service. */
    readonly connect: (spaceId: string, subject?: Subject) => Promise<SecretClient>;

    /** Reach vaults through the given connections. */
    constructor(connect: RemoteVault["connect"]) {
        this.connect = connect;
    }

    /** Create the secret with the value as its first version, once, answering its reference. */
    async write(written: Parameters<Vault["write"]>[0], caller: Subject): Promise<ObjectReference> {
        // create the secret or read the one an earlier write created
        const parent = requireType(written.vault, vault);
        const client = await this.connect(parent.scope, caller);
        const { id } = written;
        const spaceId = parent.scope;
        const existing =
            (await absent(
                client.secret.create({
                    spaceId,
                    parentId: schema.identifier("vault").parse(parent.id),
                    id,
                    name: written.name,
                    requestId: RequestId.create(),
                }),
                "CONFLICT",
            )) ?? (await client.secret.get({ spaceId, id }));

        // keep the value as the first version once
        if (existing.currentVersion === null) {
            await client.version.create({
                spaceId,
                parentId: id,
                value: { encoding: "text", value: written.value },
                requestId: RequestId.create(),
            });
        }

        return secret.reference(spaceId, id);
    }

    /** Read the secret's current text value as a caller. */
    async read(reference: ObjectReference, caller: Subject): Promise<string> {
        // read the current version
        const { scope, id } = requireType(reference, secret);
        const client = await this.connect(scope, caller);
        const { value } = await client.secret.read({
            spaceId: scope,
            id: schema.identifier("secret").parse(id),
        });
        if (value.encoding !== "text") {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `secret ${id} has no text credential`,
            });
        }

        return value.value;
    }

    /** Read the vault keeping the secret, as the account service. */
    async parent(reference: ObjectReference): Promise<ObjectReference> {
        // read the secret's vault
        const { scope, id } = requireType(reference, secret);
        const client = await this.connect(scope);
        const kept = await client.secret.get({
            spaceId: scope,
            id: schema.identifier("secret").parse(id),
        });

        return vault.reference(scope, kept.parentId);
    }

    /** Delete the secret and purge its values as the account service, doing nothing for a secret already gone. */
    async destroy(reference: ObjectReference): Promise<void> {
        // read the secret and skip one already missing or purged
        const { scope, id } = requireType(reference, secret);
        const client = await this.connect(scope);
        const key = { spaceId: scope, id: schema.identifier("secret").parse(id) };
        const existing = await absent(client.secret.get(key), "NOT_FOUND");
        if (existing === undefined || existing.purgedAt !== null) {
            return;
        }

        // trash the secret once and purge its values
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

/** Require a reference to an object of a vault type, refusing any other object. */
function requireType(
    reference: ObjectReference,
    type: typeof secret | typeof vault,
): ObjectReference {
    if (!type.is(reference)) {
        throw new ServiceError("BAD_REQUEST", {
            message: `${reference.type} ${reference.id} is no ${type.name} of the vault service`,
        });
    }

    return reference;
}

/** Await a vault call, answering undefined for the one expected failure. */
async function absent<Value>(
    pending: Promise<Value>,
    code: "CONFLICT" | "NOT_FOUND",
): Promise<Value | undefined> {
    try {
        return await pending;
    } catch (error) {
        // answer the expected failure and rethrow the others
        if (isServiceError(error) && error.code === code) {
            return undefined;
        }
        throw error;
    }
}
