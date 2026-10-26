import { Keychain } from "@destack/identity";
import { TokenCache } from "@destack/access/token";
import type { Login } from "./login.ts";

/** The person of a login at its issuer, keying their keychain entry. */
type LoginKey = Pick<Login["row"], "issuer" | "subjectId">;

/** The keychain service storing the refresh tokens of people signed in on a native device. */
export const SESSIONS = "app.destack.login";

/** The sessions of people signed in here: refresh tokens in a keychain, access tokens in memory. */
export class SessionStore {
    /** The keychain storing the sessions. */
    readonly keychain: Keychain;
    /** The access tokens refreshed and exchanged from each session, by its keychain entry. */
    readonly #tokensBySession = new Map<string, TokenCache>();

    /** Store sessions in a keychain. */
    constructor(keychain: Keychain) {
        this.keychain = keychain;
    }

    /** Read a login's session, absent after sign-out. */
    load(login: LoginKey): Promise<string | undefined> {
        return this.keychain.load(SessionStore.#entry(login));
    }

    /** Replace a login's session read before with a new one, or adopt a racing sign-in's. */
    async replace(login: LoginKey, token: string): Promise<string> {
        // replace the session read before or adopt the racing sign-in's session
        const entry = SessionStore.#entry(login);
        const current = await this.keychain.load(entry);

        return Keychain.update(this.keychain, entry, (existing) =>
            existing === current || existing === undefined ? token : existing,
        );
    }

    /** Read the access tokens of a login's session, one cache per session for every caller. */
    accessTokens(login: LoginKey): TokenCache {
        // reuse the session's cache, else start it
        const entry = SessionStore.#entry(login);
        const known = this.#tokensBySession.get(entry);
        if (known !== undefined) {
            return known;
        }
        const tokens = new TokenCache();
        this.#tokensBySession.set(entry, tokens);

        return tokens;
    }

    /** Forget a login's session, whichever one the keychain stores, and its access tokens. */
    forget(login: LoginKey): Promise<void> {
        // forget the access tokens, then the session
        const entry = SessionStore.#entry(login);
        this.#tokensBySession.delete(entry);

        return this.keychain.remove(entry);
    }

    /** Forget a login's session while it is the one read before, and report whether it did. */
    async remove(login: LoginKey, token: string): Promise<boolean> {
        // forget the session and, once forgotten, its access tokens
        const entry = SessionStore.#entry(login);
        const isRemoved = await this.keychain.removeIf(entry, token);
        if (isRemoved) {
            this.#tokensBySession.delete(entry);
        }

        return isRemoved;
    }

    /** Find a login's keychain entry by the full issuer. */
    static #entry(login: LoginKey): string {
        return `${login.subjectId}@${login.issuer}`;
    }
}
