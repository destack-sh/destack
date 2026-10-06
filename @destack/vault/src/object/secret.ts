import { Ciphertext } from "@destack/identity";
import type { VaultKey } from "../key/key.ts";
import { MAX_VALUE_BYTES, SecretGeneration, SecretValue } from "../secret/index.ts";
import {
    none,
    permission,
    principal,
    relation,
    union,
    type Creation,
    Caller,
} from "@destack/access";
import {
    and,
    check,
    type DatabaseConnection,
    eq,
    foreignKey,
    type Select,
    sql,
    unique,
    uniqueIndex,
    type TableConstraint,
} from "@destack/db";
import { defineObject, field, type Call } from "@destack/object";
import type { Service } from "@destack/service";
import type { ClientOptions } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";
import { space } from "@destack/space/object";
import { VaultKind } from "../declare/kind.ts";
import { SpaceSecret } from "../declare/space.ts";
import {
    SecretDisclosure,
    SecretPromotion,
    SecretReading,
    SecretSelection,
    VersionWrite,
} from "../secret/secret.ts";

/** The protocol each secret value's encryption context names, beside its space, vault, secret and version. */
const SECRET_PROTOCOL = "@destack/vault/secret";

/** A space's vault of secrets: a resource of the vault kind. */
export const vault = defineObject({
    name: "vault",
    plural: "vaults",
    scope: space,
    provisioned: { kind: VaultKind },
    fields: {},
    permissions: { write: none() },
    administration: ["read"],
});

/** A secret in a space's vault with its versions as values. */
export const secret = defineObject({
    name: "secret",
    plural: "secrets",
    scope: space,
    declarable: { schema: SpaceSecret },
    nested: { in: vault, delete: "restrict", receive: "write" },
    recoverable: { within: { days: 30 }, by: "delete", purge: "purge", keep: "record" },
    audited: { reads: true },
    fields: {
        /** The vault-local name. */
        name: field.string(),
        /** The version unpinned reads select, absent before one is promoted. */
        currentVersion: field.integer().optional(),
        /** The time reads were disabled, absent while enabled. */
        disabledAt: field.time().optional(),
    },
    constraints: (entry): TableConstraint[] => [
        uniqueIndex("secret_vault_name")
            .on(entry.parentId, entry.name)
            .where(sql`${entry.purgedAt} IS NULL`),
        unique("secret_scope_id").on(entry.scope, entry.id),
        foreignKey({
            columns: [entry.scope, entry.parentId],
            foreignColumns: [vault.table.scope, vault.table.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [entry.id, entry.currentVersion],
            foreignColumns: [secretVersion.table.parentId, secretVersion.table.number],
        }).onDelete("restrict"),
        check("secret_name", sql`length(${entry.name}) > 0`),
    ],
    bindable: true,
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
        read: none(),
        open: union(permission("read"), relation("consumer")),
        delete: relation("custodian"),
        purge: relation("custodian"),
    },
    administration: ["create", "get", "list", "update", "disable", "enable", "delete", "purge"],
    methods: (method) => ({
        get: method.get("get"),
        list: method.list("list"),
        create: method.create("create", { fields: ["name"], creation: custody }),
        update: method.update("update", { fields: ["name"] }),
        disable: method.mutation({ permission: "disable" }),
        enable: method.mutation({ permission: "enable" }),
        promote: method.mutation({ permission: "promote", input: SecretPromotion }),
        select: method.mutation({ permission: null, isSystem: true, input: SecretPromotion }),
        /** Provision a secret an installation's declaration binds, as its space generates it. */
        provision: method.create(null, { isSystem: true, fields: ["name"] }),
        /** Write a generated first version of a secret without one, as its space installs the package declaring it. */
        generate: method.mutation({
            permission: null,
            isSystem: true,
            input: SecretGeneration.pick({ algorithm: true }),
        }),
        read: method.query({
            permission: "open",
            input: SecretSelection,
            output: SecretReading,
            audit: { details: SecretDisclosure },
        }),
    }),
});

/** One numbered, immutable value of a secret, encrypted under its vault's key. */
export const secretVersion = defineObject({
    name: "version",
    identity: "secret-version",
    plural: "versions",
    scope: space,
    nested: { in: secret, delete: "restrict", receive: "update" },
    versioned: true,
    audited: { reads: true },
    fields: {
        /** The time after which reads are refused, absent for no expiry. */
        expiresAt: field.time().optional(),
        /** The time reads were disabled, absent while enabled. */
        disabledAt: field.time().optional(),
        /** The time the ciphertext was destroyed, absent while kept. */
        destroyedAt: field.time().optional(),
        /** The value encrypted under its vault's key, absent once destroyed. */
        ciphertext: field.string(Ciphertext).sensitive().optional(),
    },
    constraints: (entry) => [
        check("version_positive", sql`${entry.number} > 0`),
        check(
            "version_expiry",
            sql`${entry.expiresAt} IS NULL OR ${entry.expiresAt} > ${entry.createdAt}`,
        ),
    ],
    bindable: true,
    permissions: {
        get: none(),
        list: none(),
        write: none(),
        read: relation("consumer"),
        disable: none(),
        enable: none(),
        destroy: none(),
    },
    administration: ["get", "list", "disable", "enable", "destroy"],
    methods: (method) => ({
        get: method.get("get"),
        list: method.list("list"),
        create: method.create("write", { fields: ["expiresAt"], input: VersionWrite }),
        /** Store a version as the system, such as the first value a space generates. */
        store: method.create(null, { isSystem: true, fields: ["expiresAt"], input: VersionWrite }),
        disable: method.mutation({ permission: "disable" }),
        enable: method.mutation({ permission: "enable" }),
        destroy: method.mutation({ permission: "destroy" }),
        purge: method.mutation({ permission: null, isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
    }),
});

/** A persisted secret version. */
export type SecretVersion = Select<typeof secretVersion.table>;

/** Find, check, encrypt and decrypt secret versions. */
export const SecretVersion = {
    /** Find one numbered version of a secret, absent when it has none. */
    async find(
        database: DatabaseConnection,
        secretId: Select<typeof secret.table>["id"],
        version: number,
    ): Promise<SecretVersion | undefined> {
        const [row] = await database
            .select()
            .from(secretVersion.table)
            .where(
                and(
                    eq(secretVersion.table.parentId, secretId),
                    eq(secretVersion.table.number, version),
                ),
            );

        return row;
    },

    /** Refuse changing a version with a destroyed value. */
    requireKept(version: SecretVersion): void {
        if (version.destroyedAt !== null) {
            throw new ServiceError("CONFLICT", { message: "secret version is destroyed" });
        }
    },

    /** Refuse reading a destroyed, disabled or expired version. */
    requireReadable(version: SecretVersion, now: number): void {
        if (
            version.destroyedAt !== null ||
            version.disabledAt !== null ||
            (version.expiresAt !== null && version.expiresAt <= now)
        ) {
            throw new ServiceError("FORBIDDEN", { message: "secret version is unavailable" });
        }
    },

    /** Encrypt a version's value under its vault's key and keep it in the version. */
    async encrypt(
        database: DatabaseConnection,
        key: VaultKey,
        owner: Select<typeof secret.table>,
        version: number,
        value: SecretValue,
    ): Promise<void> {
        // bound the decoded bytes before encrypting the transport representation
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
        let ciphertext: Ciphertext;
        try {
            ciphertext = await key.encrypt(plaintext, SecretVersion.context(owner, version));
        } finally {
            plaintext.fill(0);
        }

        // keep it in the version
        await database
            .update(secretVersion.table)
            .set({ ciphertext })
            .where(
                and(
                    eq(secretVersion.table.parentId, owner.id),
                    eq(secretVersion.table.number, version),
                ),
            );
    },

    /** Decrypt a version's value under its encryption context. */
    async decrypt(
        key: VaultKey,
        owner: Select<typeof secret.table>,
        version: SecretVersion,
    ): Promise<SecretValue> {
        // require the ciphertext
        if (version.ciphertext === null) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: "secret ciphertext is missing",
            });
        }

        // decrypt it and clear the plaintext
        const plaintext = await key.decrypt(
            version.ciphertext,
            SecretVersion.context(owner, version.number),
        );
        try {
            return SecretValue.parse(JSON.parse(new TextDecoder().decode(plaintext)));
        } finally {
            plaintext.fill(0);
        }
    },

    /** Encode the encryption context of a version's value: its space, vault, secret and version. */
    context(owner: Select<typeof secret.table>, version: number): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(
            JSON.stringify([SECRET_PROTOCOL, owner.scope, owner.parentId, owner.id, version]),
        );
    },
};

/** Relate the installation writing a secret as its custodian, acting itself or as a delegate. */
function custody(call: Call): Creation {
    const writer = Caller.principal(call.requireAuthorization().context(call.scope));

    return {
        relationships:
            writer !== undefined && principal.installation.is(writer)
                ? [{ relation: "custodian", subject: writer }]
                : [],
    };
}

/** A space's secrets and their versions, reached at the service serving them. */
export class SecretClient {
    /** The secret methods. */
    readonly secret;
    /** The version methods. */
    readonly version;

    /** Connect to the secrets and versions of a service, as the options authenticate. */
    constructor(service: Pick<Service, "package">, options: ClientOptions) {
        this.secret = secret.connect(service, options);
        this.version = secretVersion.connect(service, options);
    }
}
