import { principal, through } from "@destack/access";
import { type Select } from "@destack/db";
import { client, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A person signed in on a computer's native client, the keychain keeping their refresh token. */
export const login = defineObject({
    name: "login",
    plural: "logins",
    scope: client,
    storage: "local",
    fields: {
        /** The universe that issued the session. */
        issuer: field.string(schema.url()),
        /** The signed-in user. */
        subject: field.reference(principal.user),
        /** The client the sign-in registered in the user's account, whose key proves it locally. */
        registration: field.reference(principal.client),
        /** The user's display name at sign-in. */
        name: field.string(),
        /** Whether the sign-in is valid, or the person must sign in again. */
        status: field.enum(["authenticated", "reauthentication"]),
    },
    // let the people signed in on the client read and write them
    permissions: { read: through("client", "use"), write: through("client", "use") },
    constraints: {
        subject: { kind: "unique", on: ["scope", "issuer", "subjectId"] },
    },
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write", { fields: ["name", "status", "registrationId"] }),
    delete: method.delete("write"),
}));
/** A login as clients read it. */
export type Login = Select<typeof login.table>;
