import { none, permission, principal, relation, through, union } from "@destack/access";
import { account, organization, space } from "@destack/account/object";
import { sudo } from "@destack/account/authentication";
import { defineObject, field, type ObjectType } from "@destack/object";
import { type Duration, schema } from "@destack/schema";
import {
    SecretDisclosure,
    SecretGeneration,
    SecretReading,
    SecretSelection,
    SecretValue,
} from "../secret/secret.ts";
import { vault } from "./vault.ts";

/** The value of a new version and when its reads end. */
export const SecretVersionInput = schema.object({
    /** The value to encrypt. */
    value: schema.sensitive(SecretValue),
    /** The time after which reads are refused, in UTC epoch milliseconds, never when absent. */
    expiresAt: schema.number().int().exactOptional(),
});
/** The value of a new version and when its reads end. */
export type SecretVersionInput = schema.Infer<typeof SecretVersionInput>;

/** How long deleted secrets stay restorable. */
export const SECRET_RECOVERY: Duration = { days: 30 };

/** The states of a secret version, as Secret Manager's `SecretVersion.State` has them. */
export const VERSION_STATES = ["enabled", "disabled", "destroyed"] as const;

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
export const secret = defineObject(
    {
        name: "secret",
        plural: "secrets",
        scope: [space, account, organization],
        nested: { in: vault, delete: "trash", receive: "write" },
        recoverable: { within: SECRET_RECOVERY, by: "delete", purge: "purge", retain: "record" },
        audited: { reads: true },
        fields: {
            /** The vault-local name. */
            name: field.string(),
            /** The version a read without a number answers, its `current` alias. */
            currentVersion: field.integer().optional(),
        },
        // bind the packages reading a secret on their behalf as its consumers
        bindable: { by: "bind" },
        relations: {
            /** The installation that wrote the secret, which gets, deletes and purges it. */
            custodian: { subjects: [principal.installation], grantedBy: null },
        },
        permissions: {
            create: through("parent", "write"),
            get: relation("custodian"),
            list: none(),
            update: relation("custodian"),
            // let people reveal a value, software bound to the secret or its vault consume it, and both read it
            reveal: none(),
            consume: union(relation("consumer"), through("parent", "consume")),
            read: union(permission("reveal"), permission("consume")),
            delete: relation("custodian"),
            purge: relation("custodian"),
            bind: permission("update"),
            provision: none(),
        },
        // let an account's owners reveal a value only after a fresh step-up, every reveal audited
        elevated: { reveal: sudo },
        reserved: ["consume", "read"],
        administration: ["create", "get", "list", "update", "delete", "purge"],
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
    },
    (method) => ({
        create: method.create("create", { fields: ["name"] }),
        /** Provision a secret an installation's declaration binds, as its space generates it. */
        provision: method.create("provision", { visibility: "installation", fields: ["name"] }),
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
            visibility: "installation",
            input: SecretGeneration.pick({ algorithm: true }),
        }),
        /** Rename a secret, or point its `current` alias at a readable version. */
        update: method.update("update", { fields: ["name", "currentVersion"] }),
    }),
);

/** One numbered, immutable value of a secret, sealed at rest. */
export const secretVersion = defineObject(
    {
        name: "version",
        identity: "secret-version",
        plural: "versions",
        scope: [space, account, organization],
        nested: { in: secret, delete: "cascade", receive: "update" },
        versioned: true,
        audited: { reads: true },
        fields: {
            /** The time after which reads are refused, absent for no expiry. */
            expiresAt: field.time().optional(),
            /** The version's state: readable when enabled, empty once destroyed. */
            state: field.enum(VERSION_STATES).default("enabled"),
            /** The value bound to its secret and version, sealed at rest, absent once destroyed. */
            content: field.json(BoundValue).sensitive().optional(),
        },
        // bind the packages reading a version on their behalf as its consumers
        bindable: { by: "bind" },
        permissions: {
            get: none(),
            list: none(),
            write: through("parent", "update"),
            read: relation("consumer"),
            disable: none(),
            enable: none(),
            destroy: through("parent", "update"),
            erase: through("parent", "purge"),
            bind: through("parent", "update"),
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
                        { state: { in: ["enabled", "disabled"] }, content: { isNotNull: true } },
                        { state: { in: ["destroyed"] }, content: { isNull: true } },
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
    },
    (method) => ({
        create: method.create("write", { fields: [], input: SecretVersionInput }),
        get: method.get("get"),
        list: method.list("list"),
        disable: method.mutation({ permission: "disable" }),
        enable: method.mutation({ permission: "enable" }),
        destroy: method.mutation({ permission: "destroy" }),
        /** Destroy a version's value as its secret's purge does. */
        erase: method.mutation({ permission: "erase", visibility: "package" }),
    }),
);
