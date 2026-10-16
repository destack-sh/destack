import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { user } from "./user.ts";

/** A WebAuthn credential a user registered. */
export const passkey = defineObject(
    {
        name: "passkey",
        plural: "passkeys",
        scope: user,
        fields: {
            /** The user-selected credential name. */
            name: field.string().optional(),
            /** The encoded WebAuthn public key. */
            publicKey: field.string(),
            /** The user authenticated by this credential. */
            user: field.reference(user, { delete: "cascade" }),
            /** The authenticator-assigned credential identifier. */
            credentialID: field.string(),
            /** The latest authenticator signature counter. */
            counter: field.integer(),
            /** The WebAuthn single-device or multi-device credential type. */
            deviceType: field.string(),
            /** Whether the credential is backed up by its authenticator. */
            backedUp: field.boolean(),
            /** The encoded authenticator transports. */
            transports: field.string().optional(),
            /** The authenticator model identifier. */
            aaguid: field.string().optional(),
        },
        permissions: ["read", "list"],
        constraints: {
            credential: { kind: "unique", on: ["credentialID"] },
            user: { kind: "index", on: ["userId"] },
        },
    },
    (method) => ({ list: method.list("list") }),
);
/** A persisted WebAuthn credential. */
export type Passkey = Select<typeof passkey.table>;
