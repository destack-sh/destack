import type { InstanceOf } from "@destack/object";
import {
    none,
    permission,
    principal,
    principalOf,
    relation,
    union,
    type Creation,
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
import { defineObject, field, method, type Call } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { installation, resource, space } from "@destack/space/object";
import { SpaceSecret } from "../declare/space.ts";
import {
    SecretDisclosure,
    SecretPromotion,
    SecretReading,
    SecretSelection,
    VersionWrite,
} from "../secret/secret.ts";

/** A space's vault: the facet of a vault resource with its secrets. */
export const vault = defineObject({
    name: "vault",
    identity: "resource",
    plural: "vaults",
    scope: space,
    fields: {
        /** The fixed resource kind of the underlying resource. */
        kind: field.enum(["vault"]).default("vault"),
    },
    constraints: (entry) => [
        unique("vault_scope_id").on(entry.scope, entry.id),
        foreignKey({
            columns: [entry.scope, entry.id, entry.kind],
            foreignColumns: [resource.table.scope, resource.table.id, resource.table.kind],
        }).onDelete("restrict"),
    ],
    permissions: { get: none(), list: none(), write: none() },
    administration: ["get", "list"],
    methods: {
        get: method.get("get"),
        list: method.list("list"),
        create: method.create(null, { isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
    },
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
        foreignKey({
            columns: [entry.scope, entry.managerInstallationId],
            foreignColumns: [installation.table.scope, installation.table.id],
        }),
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
    relations: {
        /** Installations whose live deployments captured a version of the secret. */
        reader: { subjects: [principal.installation], grantedBy: null },
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
        open: union(permission("read"), relation("reader")),
        delete: relation("custodian"),
        purge: relation("custodian"),
    },
    administration: ["create", "get", "list", "update", "disable", "enable", "delete", "purge"],
    methods: {
        get: method.get("get"),
        list: method.list("list"),
        create: method.create("create", { fields: ["name"], creation: custody }),
        update: method.update("update", { fields: ["name"] }),
        disable: method({ permission: "disable" }),
        enable: method({ permission: "enable" }),
        promote: method({ permission: "promote", input: SecretPromotion }),
        select: method({ permission: null, isSystem: true, input: SecretPromotion }),
        read: method({
            permission: "open",
            mutates: false,
            input: SecretSelection,
            output: SecretReading,
            audit: { details: SecretDisclosure },
        }),
    },
});

/** One numbered, immutable value of a secret with its ciphertext kept apart. */
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
    },
    constraints: (entry) => [
        check("version_positive", sql`${entry.number} > 0`),
        check(
            "version_expiry",
            sql`${entry.expiresAt} IS NULL OR ${entry.expiresAt} > ${entry.createdAt}`,
        ),
    ],
    relations: {
        /** Installations whose live deployments captured the version. */
        reader: { subjects: [principal.installation], grantedBy: null },
    },
    permissions: {
        get: none(),
        list: none(),
        write: none(),
        read: relation("reader"),
        disable: none(),
        enable: none(),
        destroy: none(),
    },
    administration: ["get", "list", "disable", "enable", "destroy"],
    methods: {
        get: method.get("get"),
        list: method.list("list"),
        create: method.create("write", { fields: ["expiresAt"], input: VersionWrite }),
        disable: method({ permission: "disable" }),
        enable: method({ permission: "enable" }),
        destroy: method({ permission: "destroy" }),
        purge: method({ permission: null, isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
    },
});

/** A persisted secret version. */
export type SecretVersion = Select<typeof secretVersion.table>;

/** The versions of secrets: found by number, and checked before they change or are read. */
export const SecretVersion = {
    /** Find one numbered version of a secret, absent when it has none. */
    async find(
        database: DatabaseConnection,
        secretId: InstanceOf<typeof secret>["id"],
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
};

/** Relate the installation writing a secret as its custodian, acting itself or as a delegate. */
function custody(call: Call): Creation {
    const writer = principalOf(call.served().context(call.scope));

    return {
        relationships:
            writer !== undefined && principal.installation.is(writer)
                ? [{ relation: "custodian", subject: writer }]
                : [],
    };
}
