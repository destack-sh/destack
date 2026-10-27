import { through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { PublicKey } from "@destack/identity";
import { installation } from "@destack/object";
import { space } from "@destack/account/object";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";

/** The longest installation key name, a label such as a CI system's name. */
const MAX_KEY_NAME_LENGTH = 200;

/** A public key of an installation. */
export const installationKey = defineObject(
    {
        name: "installation-key",
        plural: "installationKeys",
        scope: space,
        nested: { in: installation, receive: "update", delete: "cascade" },
        fields: {
            /** The installation-local name, such as the CI system that stores the private half. */
            name: field.string(schema.string().min(1).max(MAX_KEY_NAME_LENGTH)),
            /** The public half. */
            publicKey: field.json(PublicKey),
            /** The principal who registered the key. */
            createdBy: field.subject().caller(),
            /** The time the key stops authenticating, absent for a key that stays until revoked. */
            expiresAt: field.time().optional(),
            /** The time the key last granted a token. */
            lastUsedAt: field.time().optional(),
            /** The time the key was revoked. */
            revokedAt: field.time().optional(),
        },
        permissions: {
            read: through("parent", "read"),
            register: through("parent", "update"),
            revoke: through("parent", "update"),
            record: through("parent", "token"),
        },
        constraints: {
            expiry: {
                kind: "check",
                condition: {
                    expiresAt: { OR: [{ isNull: true }, { gt: { field: "createdAt" } }] },
                },
            },
        },
    },
    (method) => ({
        create: method.create("register", {
            fields: ["name", "publicKey", "expiresAt"],
            isPredicted: false,
        }),
        revoke: method.mutation({ permission: "revoke" }),
        use: method.mutation({ permission: "record", visibility: "installation" }),
    }),
).implement({
    methods: {
        create: async (call, next) => {
            // refuse a key past its expiry or off its curve
            const { expiresAt, publicKey } = call.input;
            if (expiresAt !== undefined && expiresAt !== null && expiresAt <= call.now) {
                throw StackError.of(
                    "EXPIRY_IN_PAST",
                    "installation key expiry must be in the future",
                );
            }
            await PublicKey.import(publicKey, call.environment.crypto);

            return next();
        },
        revoke: async (call) => {
            // refuse revoking a key twice
            if (call.target.revokedAt !== null) {
                throw StackError.of("KEY_REVOKED", "installation key is revoked");
            }

            return call.update({ revokedAt: call.now });
        },
        use: (call) => call.update({ lastUsedAt: call.now }),
    },
});

/** A persisted installation key. */
export type InstallationKey = Select<typeof installationKey.table>;
