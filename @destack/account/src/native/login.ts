import { principal, through } from "@destack/access";
import { type Select } from "@destack/db";
import type { Environment } from "@destack/environment";
import { defineObject, device, field, type Fetch } from "@destack/object";
import { type Bookmark } from "@destack/object/client";
import { schema } from "@destack/schema";
import type { TokenCache } from "@destack/access/token";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import { Issuer } from "../client/issuer.ts";
import { Session } from "../client/session.ts";
import { UserIdentity } from "../client/user.ts";

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
/** The source of a login's account tokens and client proofs: the keychain, or its machine. */
export interface LoginCredential {
    /** Read the cache of a login's access tokens, by package and binding. */
    readonly tokens: (row: Login["row"]) => TokenCache;
    /** Issue a login's account access token anew. */
    readonly issue: (signedIn: Login) => Promise<string>;
    /** Prove the login's client to this computer's machine. */
    readonly prove: (signedIn: Login) => Promise<string>;
    /** Forget a signed-out login: its refresh token and its login here. */
    readonly forget: (row: Login["row"]) => Promise<void>;
}

/** A person signed in on a computer: the device's record of their sign-in and its credential. */
export class Login {
    /** The login's row. */
    readonly row: Select<typeof login.table>;
    /** The user's calls to packages, with their tokens. */
    readonly session: Session<UserIdentity>;
    /** The credential issuing the person's account tokens and client proofs. */
    readonly #credential: LoginCredential;

    /** Act as the person of a login through a credential. */
    constructor(
        row: Login["row"],
        credential: LoginCredential,
        fetch: Fetch,
        environment: Environment,
        bookmark: Bookmark,
    ) {
        // require a person still signed in
        if (row.status !== "authenticated") {
            throw AccountError.of(
                "SIGN_IN_UNAUTHENTICATED",
                "authentication required: the person is not signed in",
            );
        }

        // act as the user through the credential's tokens at the login's issuer
        this.row = row;
        this.#credential = credential;
        const user = new UserIdentity(
            schema.identifier("user").parse(row.subjectId),
            new Issuer(row.issuer, fetch, environment, bookmark),
            { cache: credential.tokens(row), issue: () => credential.issue(this) },
            environment,
        );
        this.session = new Session(user, {
            universe: new URL(row.issuer).origin,
            send: fetch,
            bookmark,
            environment,
        });
    }

    /** The signed-in user's tokens. */
    get user(): UserIdentity {
        return this.session.credential;
    }

    /** Prove the person's client to this computer's machine. */
    prove(): Promise<string> {
        return this.#credential.prove(this);
    }
}
