import { principal, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, device, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A person signed in on a computer's native device, the keychain storing their refresh token. */
export const login = defineObject({
    name: "login",
    plural: "logins",
    scope: device,
    storage: "local",
    fields: {
        /** The universe that issued the session. */
        issuer: field.string(schema.url()).immutable(),
        /** The signed-in user. */
        subject: field.reference(principal.user).immutable(),
        /** The client the sign-in registered in the user's account, whose key proves it locally. */
        client: field.reference(principal.client),
        /** The user's display name at sign-in. */
        name: field.string(),
        /** Whether the sign-in is valid, or the person must sign in again. */
        status: field.enum(["authenticated", "reauthentication"]),
    },
    // let the people signed in on the device read and write them
    permissions: { read: through("device", "use"), write: through("device", "use") },
    constraints: {
        subject: { kind: "unique", on: ["scope", "issuer", "subjectId"] },
    },
});
/** A login as devices read it. */
export type Login = Select<typeof login.table>;
