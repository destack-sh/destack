import { none, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { PublicKey } from "@destack/identity";
import { CallError } from "@destack/object";
import { installation } from "./installation.ts";
import { space } from "@destack/account/object";

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
            /** The installation-local name, such as the CI system that keeps the private half. */
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
            record: none(),
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
        use: method.mutation({ permission: "record", isInternal: true }),
    }),
).implement({
    methods: {
        create: async (call, next) => {
            // refuse a key past its expiry or off its curve
            const { expiresAt, publicKey } = call.input;
            if (expiresAt !== undefined && expiresAt !== null && expiresAt <= call.now) {
                throw new CallError("BAD_REQUEST", {
                    message: "installation key expiry must lie in the future",
                });
            }
            await PublicKey.import(publicKey);

            return next();
        },
        revoke: async (call) => {
            // refuse revoking a key twice
            if (call.target.revokedAt !== null) {
                throw new CallError("CONFLICT", { message: "installation key is revoked" });
            }

            return call.update({ revokedAt: call.now });
        },
        use: (call) => call.update({ lastUsedAt: call.now }),
    },
});

/** A persisted installation key. */
export type InstallationKey = Select<typeof installationKey.table>;
