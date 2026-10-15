import { none, permission, principal, relation, through, union } from "@destack/access";
import { account, organisation, space } from "@destack/account/object";
import { sudo } from "@destack/account/authentication";
import { type RowImage, type Select, type Snapshot } from "@destack/db";
import { type Call, defineObject, field, type ObjectType } from "@destack/object";
import { type Duration, type Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Binding } from "@destack/stack/object";
import { SecretDefinition } from "../declare/stack.ts";
import {
    MAX_VALUE_BYTES,
    SecretDisclosure,
    SecretGeneration,
    SecretReading,
    SecretSelection,
    SecretValue,
} from "../secret/secret.ts";
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

/** How long deleted secrets stay restorable. */
export const SECRET_RECOVERY: Duration = { days: 30 };

/** A version's stored value with its secret and version, so a copied value reads as foreign. */
export const BoundValue = schema.object({
    /** The secret the value was written for. */
    secret: schema.identifier("secret"),
    /** The version the value was written for. */
    version: schema.identifier("secret-version"),
    /** The value. */
    value: SecretValue,
});
/** A version's stored value with its secret and version. */
export type BoundValue = schema.Infer<typeof BoundValue>;

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
    },
    // bind the packages reading a secret on their behalf as its consumers
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
        promote: permission("update"),
        // let people reveal a value, software bound to the secret or its vault consume it, and both read it
        reveal: none(),
        consume: union(relation("consumer"), through("parent", "consume")),
        read: union(permission("reveal"), permission("consume")),
        delete: relation("custodian"),
        purge: relation("custodian"),
        bind: none(),
        provision: none(),
    },
    // let an account's owners reveal a value only after a fresh step-up, every reveal audited
    elevated: { reveal: sudo },
    reserved: ["consume", "read"],
    administration: ["create", "get", "list", "update", "promote", "delete", "purge"],
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
    create: method.create("create", { fields: ["name"] }),
    /** Provision a secret an installation's declaration binds, as its space generates it. */
    provision: method.create("provision", { isInternal: true, fields: ["name"] }),
    get: method.get("get"),
    list: method.list("list"),
    read: method.query({
        permission: "read",
        input: SecretSelection,
        output: SecretReading,
        audit: { details: SecretDisclosure },
    }),
    /** Write a generated first version of a secret without one. */
    generate: method.mutation({
        permission: "provision",
        isInternal: true,
        input: SecretGeneration.pick({ algorithm: true }),
    }),
    /** Select a readable version as current, as a version's creation does for its writer. */
    promote: method.mutation({ permission: "promote", input: SecretPromotion }),
    update: method.update("update", { fields: ["name"] }),
}));

/** One numbered, immutable value of a secret, sealed at rest. */
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
        /** The time the value was destroyed, absent while intact. */
        destroyedAt: field.time().optional(),
        /** The value bound to its secret and version, sealed at rest, absent once destroyed. */
        content: field.json(BoundValue).sensitive().optional(),
    },
    // bind the packages reading a version on their behalf as its consumers
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
    // let only consumers read a version, so a person reveals values through the secret's step-up
    reserved: ["read"],
    administration: ["get", "list", "disable", "enable", "destroy"],
    constraints: {
        positive: { kind: "check", condition: { number: { gt: 0 } } },
        sealed: {
            kind: "check",
            condition: {
                OR: [
                    { destroyedAt: { isNull: true }, content: { isNotNull: true } },
                    { destroyedAt: { isNotNull: true }, content: { isNull: true } },
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
    create: method.create("write", { fields: ["expiresAt"], input: SecretVersionInput }),
    get: method.get("get"),
    list: method.list("list"),
    disable: method.mutation({ permission: "disable" }),
    enable: method.mutation({ permission: "enable" }),
    destroy: method.mutation({ permission: "destroy" }),
    /** Destroy a version's value as its secret's purge does. */
    erase: method.mutation({ permission: "erase", isInternal: true }),
    delete: method.delete("delete", { isInternal: true }),
}));

/** A persisted secret version. */
export type SecretVersion = Select<typeof secretVersion.table>;

/** Find, check and read secret versions. */
export const SecretVersion = {
    /** Read the secret owning a version, refusing an unknown one. */
    async owner(call: Call, id: Identifier<"secret">): Promise<RowImage<typeof secret.table>> {
        const row = await call.read(secret).findFirst({ where: { id } });
        if (row === undefined) {
            throw new CallError("NOT_FOUND", { message: "secret not found" });
        }

        return row;
    },

    /** Read a secret's selected version, its current one by default, as a caller may. */
    async read(
        target: RowImage<typeof secret.table>,
        version: number | undefined,
        call: Call,
    ): Promise<SecretReading> {
        // refuse a secret without a selected version
        const number = version ?? target.currentVersion;
        if (number === null) {
            throw new CallError("NOT_FOUND", { message: "secret has no current version" });
        }

        // let software bound to the secret, its vault or the selected version read it
        const selected = await SecretVersion.find(call, target.id, number);
        const consumes = await call.check(
            secret.permission("consume"),
            secret.reference(call.scope, target.id),
        );
        const isConsumer =
            consumes.isAllowed ||
            (selected !== undefined &&
                (
                    await call.check(
                        secretVersion.permission("read"),
                        secretVersion.reference(call.scope, selected.id),
                    )
                ).isAllowed);

        // require anyone else to reveal through the secret, challenging for a step-up
        if (!isConsumer) {
            await call.require(
                secret.permission("reveal"),
                secret.reference(call.scope, target.id),
            );
        }
        // answer a missing version only to those revealing the secret
        if (selected === undefined) {
            throw new CallError("NOT_FOUND", { message: "secret version not found" });
        }

        return {
            version: selected.number,
            value: SecretVersion.requireReadable(selected, call.now),
        };
    },

    /** Refuse destroying a version already destroyed or one a live deployment runs with. */
    async requireReleased(reader: Pick<Snapshot, "read">, version: SecretVersion): Promise<void> {
        SecretVersion.requireIntact(version);
        await Binding.requireReleased(reader, {
            type: secret.name,
            scope: version.scope,
            id: version.parentId,
            version: version.number,
        });
    },

    /** Write the fields of a version whose value is destroyed. */
    erasure(now: number) {
        return { destroyedAt: now, content: null };
    },

    /** Find one numbered version of a secret, absent when it has none. */
    async find(
        call: Pick<Call, "reveal">,
        secretId: Select<typeof secret.table>["id"],
        version: number,
    ): Promise<SecretVersion | undefined> {
        const [row] = await call.reveal(secretVersion).findMany({
            where: {
                parentId: secretId,
                number: version,
            },
        });

        return row;
    },

    /** Refuse changing a version with a destroyed value. */
    requireIntact(version: SecretVersion): void {
        if (version.destroyedAt !== null) {
            throw new CallError("CONFLICT", { message: "secret version is destroyed" });
        }
    },

    /** Refuse reading a destroyed, disabled, expired or foreign version, answering its value. */
    requireReadable(version: SecretVersion, now: number): SecretValue {
        const { content } = version;
        // refuse a value copied from another version or secret
        if (
            content !== null &&
            (content.version !== version.id || content.secret !== version.parentId)
        ) {
            throw new CallError("FORBIDDEN", {
                message: `secret version ${version.id} holds the value of another version`,
            });
        }
        // refuse a destroyed, disabled or expired version
        else if (
            content === null ||
            version.disabledAt !== null ||
            (version.expiresAt !== null && version.expiresAt <= now)
        ) {
            throw new CallError("FORBIDDEN", { message: "secret version is unavailable" });
        }

        return content.value;
    },

    /** Refuse a value beyond the decoded byte limit. */
    requireSize(value: SecretValue): void {
        // measure the decoded bytes, wiping them once counted
        const bytes =
            value.encoding === "text"
                ? new TextEncoder().encode(value.value)
                : Uint8Array.fromBase64(value.value);
        const size = bytes.byteLength;
        bytes.fill(0);
        if (size > MAX_VALUE_BYTES) {
            throw new CallError("PAYLOAD_TOO_LARGE", {
                message: `secret value exceeds ${MAX_VALUE_BYTES} bytes`,
            });
        }
    },
};
