import { none, permission, principal, relation, through, union } from "@destack/access";
import { account, organisation, space } from "@destack/account/object";
import { type Select, type Transaction } from "@destack/db";
import { Ciphertext } from "@destack/identity";
import { defineObject, field, type ObjectType } from "@destack/object";
import { type Duration, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { SecretDefinition } from "../declare/stack.ts";
import type { VaultKey } from "../key/key.ts";
import {
    MAX_VALUE_BYTES,
    SecretDisclosure,
    SecretGeneration,
    SecretReading,
    SecretSelection,
    SecretValue,
} from "../secret/secret.ts";
import { vaultKey } from "./key.ts";
import { vault } from "./vault.ts";

/** The stage a new version takes, as AWS Secrets Manager stages versions. */
export const VERSION_STAGES = ["current", "pending"] as const;

/** The value of a new version and the stage it takes. */
export const SecretVersionInput = schema.object({
    /** The value to encrypt. */
    value: schema.sensitive(SecretValue),
    /** The stage the version takes, current when absent. */
    stage: schema.enum(VERSION_STAGES).exactOptional(),
});
/** The value of a new version and the stage it takes. */
export type SecretVersionInput = schema.Infer<typeof SecretVersionInput>;

/** The version a promotion selects as its secret's current one. */
export const SecretPromotion = schema.object({
    /** The version's number within its secret. */
    version: schema.number().int().positive(),
});
/** The version a promotion selects as its secret's current one. */
export type SecretPromotion = schema.Infer<typeof SecretPromotion>;

/** How long deleted secrets stay restorable by default. */
export const SECRET_RECOVERY: Duration = { days: 30 };

/** The protocol each secret value's encryption context starts with. */
const SECRET_PROTOCOL = "@destack/vault/secret";

/** A secret in a space's vault with its versions as values. */
export const secret = defineObject({
    name: "secret",
    plural: "secrets",
    scope: [space, account, organisation],
    declarable: { schema: SecretDefinition },
    nested: { in: vault, delete: "restrict", receive: "write" },
    recoverable: { within: SECRET_RECOVERY, by: "delete", purge: "purge", keep: "record" },
    audited: { reads: true },
    fields: {
        /** The vault-local name. */
        name: field.string(),
        /** The version reads without a number select, absent before the first version. */
        currentVersion: field.integer().optional(),
        /** The time reads were disabled, absent while enabled. */
        disabledAt: field.time().optional(),
    },
    // bind the machine services reading a secret on their behalf as its consumers
    bindable: { by: "bind" },
    relations: {
        /** The installation that wrote the secret, which gets, deletes and purges it. */
        custodian: { subjects: [principal.installation], grantedBy: null },
    },
    permissions: {
        create: none(),
        get: relation("custodian"),
        list: none(),
        update: none(),
        disable: none(),
        enable: none(),
        promote: none(),
        select: permission("update"),
        read: none(),
        open: union(permission("read"), relation("consumer")),
        delete: relation("custodian"),
        purge: relation("custodian"),
        bind: none(),
        provision: none(),
    },
    administration: [
        "create",
        "get",
        "list",
        "update",
        "disable",
        "enable",
        "promote",
        "delete",
        "purge",
    ],
    constraints: {
        vaultName: {
            kind: "index",
            on: ["parentId", "name"],
            unique: true,
            where: { purgedAt: { isNull: true } },
        },
        scopeId: { kind: "unique", on: ["scope", "id"] },
        vault: {
            kind: "foreignKey",
            on: ["scope", "parentId"],
            references: { object: () => vault, on: ["scope", "id"] },
            onDelete: "no action",
        },
        secretVersion: {
            kind: "foreignKey",
            on: ["id", "currentVersion"],
            references: { object: (): ObjectType => secretVersion, on: ["parentId", "number"] },
            onDelete: "no action",
        },
        name: { kind: "check", condition: { name: { length: { gt: 0 } } } },
    },
}).methods((method) => ({
    get: method.get("get"),
    list: method.list("list"),
    create: method.create("create", { fields: ["name"] }),
    update: method.update("update", { fields: ["name"] }),
    disable: method.mutation({ permission: "disable" }),
    enable: method.mutation({ permission: "enable" }),
    promote: method.mutation({ permission: "promote", input: SecretPromotion }),
    /** Select a new version as current, as its creation does for its writer. */
    select: method.mutation({
        permission: "select",
        isInternal: true,
        input: SecretPromotion,
    }),
    /** Provision a secret an installation's declaration binds, as its space generates it. */
    provision: method.create("provision", { isInternal: true, fields: ["name"] }),
    /** Write a generated first version of a secret without one. */
    generate: method.mutation({
        permission: "provision",
        isInternal: true,
        input: SecretGeneration.pick({ algorithm: true }),
    }),
    read: method.query({
        permission: "open",
        input: SecretSelection,
        output: SecretReading,
        audit: { details: SecretDisclosure },
    }),
}));

/** One numbered, immutable value of a secret, encrypted under a version of its vault's key. */
export const secretVersion = defineObject({
    name: "version",
    identity: "secret-version",
    plural: "versions",
    scope: [space, account, organisation],
    nested: { in: secret, delete: "restrict", receive: "update" },
    versioned: true,
    audited: { reads: true },
    fields: {
        /** The time after which reads are refused, absent for no expiry. */
        expiresAt: field.time().optional(),
        /** The time reads were disabled, absent while enabled. */
        disabledAt: field.time().optional(),
        /** The time the ciphertext was destroyed, absent while intact. */
        destroyedAt: field.time().optional(),
        /** The vault key version encrypting the value, absent once destroyed. */
        key: field.reference(vaultKey, { delete: "restrict" }).optional(),
        /** The value encrypted under its key version, absent once destroyed. */
        ciphertext: field.string(Ciphertext).sensitive().optional(),
    },
    // bind the machine services reading a version on their behalf as its consumers
    bindable: { by: "bind" },
    permissions: {
        get: none(),
        list: none(),
        write: none(),
        read: relation("consumer"),
        disable: none(),
        enable: none(),
        destroy: none(),
        erase: through("parent", "purge"),
        bind: none(),
        delete: none(),
    },
    administration: ["get", "list", "disable", "enable", "destroy"],
    constraints: {
        positive: { kind: "check", condition: { number: { gt: 0 } } },
        sealed: {
            kind: "check",
            condition: {
                AND: [
                    {
                        OR: [
                            { destroyedAt: { isNull: true }, ciphertext: { isNotNull: true } },
                            { destroyedAt: { isNotNull: true }, ciphertext: { isNull: true } },
                        ],
                    },
                    {
                        OR: [
                            { ciphertext: { isNull: true }, keyId: { isNull: true } },
                            { ciphertext: { isNotNull: true }, keyId: { isNotNull: true } },
                        ],
                    },
                ],
            },
        },
        expiry: {
            kind: "check",
            condition: {
                expiresAt: { OR: [{ isNull: true }, { gt: { field: "createdAt" } }] },
            },
        },
    },
}).methods((method) => ({
    get: method.get("get"),
    list: method.list("list"),
    create: method.create("write", { fields: ["expiresAt"], input: SecretVersionInput }),
    disable: method.mutation({ permission: "disable" }),
    enable: method.mutation({ permission: "enable" }),
    destroy: method.mutation({ permission: "destroy" }),
    /** Destroy a version's value as its secret's purge does. */
    erase: method.mutation({ permission: "erase", isInternal: true }),
    delete: method.delete("delete", { isInternal: true }),
}));

/** A persisted secret version. */
export type SecretVersion = Select<typeof secretVersion.table>;

/** Find, check, encrypt and decrypt secret versions. */
export const SecretVersion = {
    /** Find one numbered version of a secret, absent when it has none. */
    async find(
        transaction: Transaction,
        secretId: Select<typeof secret.table>["id"],
        version: number,
    ): Promise<SecretVersion | undefined> {
        const [row] = await transaction.rows(secretVersion.table, {
            parentId: secretId,
            number: version,
        });

        return row;
    },

    /** Refuse changing a version with a destroyed value. */
    requireIntact(version: SecretVersion): void {
        if (version.destroyedAt !== null) {
            throw new ServiceError("CONFLICT", { message: "secret version is destroyed" });
        }
    },

    /** Refuse reading a destroyed, disabled or expired version, answering its sealed value. */
    requireReadable(
        version: SecretVersion,
        now: number,
    ): { readonly keyId: string; readonly ciphertext: Ciphertext } {
        const { keyId, ciphertext } = version;
        if (
            keyId === null ||
            ciphertext === null ||
            version.disabledAt !== null ||
            (version.expiresAt !== null && version.expiresAt <= now)
        ) {
            throw new ServiceError("FORBIDDEN", { message: "secret version is unavailable" });
        }

        return { keyId, ciphertext };
    },

    /** Seal a version's value under a vault key version, bound to the version's identity. */
    async seal(
        key: VaultKey,
        owner: Select<typeof secret.table>,
        versionId: string,
        value: SecretValue,
    ): Promise<Ciphertext> {
        // refuse a value beyond the decoded byte limit
        const bytes =
            value.encoding === "text"
                ? new TextEncoder().encode(value.value)
                : Uint8Array.fromBase64(value.value);
        const size = bytes.byteLength;
        bytes.fill(0);
        if (size > MAX_VALUE_BYTES) {
            throw new ServiceError("PAYLOAD_TOO_LARGE", {
                message: `secret value exceeds ${MAX_VALUE_BYTES} bytes`,
            });
        }

        // encrypt the value under the version's encryption context
        const plaintext = new TextEncoder().encode(JSON.stringify(value));
        try {
            return await key.encrypt(plaintext, SecretVersion.context(owner, versionId));
        } finally {
            plaintext.fill(0);
        }
    },

    /** Open a version's sealed value under the key version that sealed it. */
    async open(
        key: VaultKey,
        owner: Select<typeof secret.table>,
        versionId: string,
        ciphertext: Ciphertext,
    ): Promise<SecretValue> {
        const plaintext = await key.decrypt(ciphertext, SecretVersion.context(owner, versionId));
        try {
            return SecretValue.parse(JSON.parse(new TextDecoder().decode(plaintext)));
        } finally {
            plaintext.fill(0);
        }
    },

    /** Encode the encryption context of a version's value: its scope, vault, secret and version. */
    context(owner: Select<typeof secret.table>, versionId: string): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(
            JSON.stringify([SECRET_PROTOCOL, owner.scope, owner.parentId, owner.id, versionId]),
        );
    },
};
