import { and, eq, not, type DatabaseConnection } from "@destack/db";
import { Order } from "@destack/db/query";
import {
    CopyStage,
    Sealed,
    type Provider,
    type Provisioning,
    type Copying,
} from "@destack/resource";
import { defineSchema, identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { secret, vault } from "../object/index.ts";
import { vaultValue } from "../stack/index.ts";
import { type Keyring, VaultKey } from "../encryption/index.ts";
import { VaultValue } from "../server/value.ts";
import { VaultKind } from "../declare/vault.ts";

/**
 * The values one chunk of a vault copy carries.
 *
 * A value is at most 64 KiB and usually far less: a hundred values fit in a few megabytes.
 */
const CHUNK_VALUES = 100;

/** The order values travel in: by secret, then version. */
const KEY_ORDER = Order.complete([], vaultValue);

/** One value's place in a vault's values in key order. */
const ValueKey = defineSchema(
    schema.object({
        /** The secret. */
        secretId: identifier("secret"),
        /** The version. */
        version: schema.number().int().positive(),
    }),
);
/** One value's place in a vault's values. */
type ValueKey = schema.Infer<typeof ValueKey>;

/** Where a vault export continues: the stage it read, and the last value it read. */
const ValueCursor = defineSchema(
    schema.object({
        /** The stage the export read. */
        stage: CopyStage,
        /** The last value read, absent at the vault's start. */
        key: ValueKey.optional(),
    }),
);

/** The values of one vault copy chunk with data keys sealed to the target. */
const ValueRange = defineSchema(
    schema.object({
        /** The value the range follows, absent from the vault's start. */
        after: ValueKey.optional(),
        /** The last value of the range, absent to the vault's end. */
        last: ValueKey.optional(),
        /** The range's values. */
        values: schema.array(
            ValueKey.extend({
                /** The envelope format. */
                format: schema.number().int(),
                /** The storage location the ciphertext authenticates. */
                location: schema.string().min(1),
                /** The base64 value ciphertext. */
                ciphertext: schema.base64(),
                /** The base64 value nonce. */
                nonce: schema.base64(),
                /** The data key, sealed to the target. */
                key: Sealed,
            }),
        ),
    }),
);
/** The values one chunk of a vault copy carries. */
type ValueRange = schema.Infer<typeof ValueRange>;

/** Provide vaults as values in the space database. */
export function vaultProvider(
    database: DatabaseConnection,
    keyring: Keyring,
    location: string,
): Provider<typeof VaultKind, typeof vault> &
    Provisioning<typeof VaultKind> &
    Copying<typeof VaultKind> {
    return {
        kind: VaultKind,
        code: "vault",
        facet: vault,
        provision: async (resource) => {
            // keep the vault's key
            await VaultKey.provision(
                database,
                keyring,
                location,
                identifier("resource").parse(resource.id),
            );

            return { reference: resource.id };
        },
        export: async function* (copy, after, signal) {
            // read again from the start once fenced
            const cursor = after === undefined ? undefined : ValueCursor.parse(JSON.parse(after));
            let from = cursor?.stage === copy.stage ? cursor.key : undefined;

            // read the vault's values in key order, sealing each data key to the target
            let isRead = false;
            while (!isRead && !signal.aborted) {
                const rows = await database
                    .select({ value: vaultValue, scope: secret.table.scope })
                    .from(vaultValue)
                    .innerJoin(secret.table, eq(secret.table.id, vaultValue.secretId))
                    .where(
                        and(
                            eq(secret.table.parentId, identifier("resource").parse(copy.record.id)),
                            from === undefined ? undefined : following(from),
                        ),
                    )
                    .orderBy(...Order.render(KEY_ORDER, vaultValue))
                    .limit(CHUNK_VALUES);
                const sealed = await Promise.all(
                    rows.map(async ({ value, scope }) => {
                        const identity = {
                            spaceId: scope,
                            vaultId: copy.record.id,
                            secretId: value.secretId,
                            version: value.version,
                        };

                        return {
                            secretId: value.secretId,
                            version: value.version,
                            ...(await VaultValue.transfer(
                                await VaultKey.load(database, keyring, location, identity.vaultId),
                                value,
                                identity,
                                copy.recipient,
                            )),
                        };
                    }),
                );

                // yield the range up to the last value read, open-ended once the vault is read
                isRead = rows.length < CHUNK_VALUES;
                const end = sealed.at(-1);
                const reached =
                    end === undefined ? from : { secretId: end.secretId, version: end.version };
                const range: ValueRange = {
                    ...(from === undefined ? {} : { after: from }),
                    ...(isRead || reached === undefined ? {} : { last: reached }),
                    values: sealed,
                };
                from = reached;
                yield {
                    cursor: JSON.stringify({
                        stage: copy.stage,
                        ...(reached === undefined ? {} : { key: reached }),
                    }),
                    body: range,
                };
            }
        },
        import: async (copy, chunk) => {
            // remove the target's other values in the range
            const range = ValueRange.parse(chunk.body);
            const present = await database
                .select({ secretId: vaultValue.secretId, version: vaultValue.version })
                .from(vaultValue)
                .innerJoin(secret.table, eq(secret.table.id, vaultValue.secretId))
                .where(
                    and(
                        eq(secret.table.parentId, identifier("resource").parse(copy.record.id)),
                        range.after === undefined ? undefined : following(range.after),
                        range.last === undefined ? undefined : through(range.last),
                    ),
                );
            const kept = new Set(range.values.map(keyOf));
            const existing = new Set(present.map(keyOf));
            await database.remove(
                vaultValue,
                present.filter((value) => !kept.has(keyOf(value))),
            );

            // encrypt each value the target lacks again under its own root key and location
            const missing = range.values.filter((value) => !existing.has(keyOf(value)));
            const resealed = await Promise.all(
                missing.map(async (value) => {
                    const identity = {
                        spaceId: copy.record.scope,
                        vaultId: copy.record.id,
                        secretId: value.secretId,
                        version: value.version,
                    };
                    const envelope = await VaultValue.receive(
                        await VaultKey.load(database, keyring, location, identity.vaultId),
                        value,
                        identity,
                        copy.recipient,
                    );

                    return { secretId: value.secretId, version: value.version, ...envelope };
                }),
            );
            if (resealed.length > 0) {
                await database.insert(vaultValue).values(resealed);
            }
        },
        destroy: async (resource) => {
            // refuse destroying a vault with secret values
            const [stored] = await database
                .select({ secretId: vaultValue.secretId })
                .from(vaultValue)
                .innerJoin(secret.table, eq(secret.table.id, vaultValue.secretId))
                .where(eq(secret.table.parentId, identifier("resource").parse(resource.id)))
                .limit(1);
            if (stored !== undefined) {
                throw new ServiceError("CONFLICT", {
                    message: `vault has values of secret ${stored.secretId}: purge its secrets first`,
                });
            }

            // delete its key
            await VaultKey.discard(database, identifier("resource").parse(resource.id));
        },
    };
}

/** Select the values after one in key order. */
function following(key: ValueKey) {
    return Order.after(KEY_ORDER, vaultValue, key);
}

/** Select the values up to and including one in key order. */
function through(key: ValueKey) {
    return not(Order.after(KEY_ORDER, vaultValue, key));
}

/** Key one value by its secret and version. */
function keyOf(value: ValueKey): string {
    return `${value.secretId}/${value.version}`;
}
