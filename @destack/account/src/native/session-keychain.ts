import { Keychain } from "@destack/identity";
import type { LoginRow } from "./login.ts";

/** The person of a login at its issuer, keying their keychain entry. */
type LoginKey = Pick<LoginRow, "issuer" | "subjectId">;

/** The keychain service storing the refresh tokens of people signed in on a native device. */
export const SESSIONS = "app.destack.login";

/** The refresh tokens of the people signed in on this computer, one keychain entry each. */
export class SessionKeychain {
    /** The keychain storing the sessions. */
    readonly keychain: Keychain;

    /** Store sessions in a keychain. */
    constructor(keychain: Keychain) {
        this.keychain = keychain;
    }

    /** Read a login's session, absent after sign-out. */
    load(login: LoginKey): Promise<string | undefined> {
        return this.keychain.load(SessionKeychain.#entry(login));
    }

    /** Replace a login's session read before with a new one, or adopt a racing sign-in's. */
    async replace(login: LoginKey, token: string): Promise<string> {
        // replace the session read before or adopt the racing sign-in's session
        const entry = SessionKeychain.#entry(login);
        const current = await this.keychain.load(entry);

        return Keychain.update(this.keychain, entry, (existing) =>
            existing === current || existing === undefined ? token : existing,
        );
    }

    /** Forget a login's session, whichever one the keychain stores. */
    forget(login: LoginKey): Promise<void> {
        return this.keychain.remove(SessionKeychain.#entry(login));
    }

    /** Forget a login's session while it is the one read before, and report whether it did. */
    remove(login: LoginKey, token: string): Promise<boolean> {
        return this.keychain.removeIf(SessionKeychain.#entry(login), token);
    }

    /** Find a login's keychain entry by the full issuer. */
    static #entry(login: LoginKey): string {
        return `${login.subjectId}@${login.issuer}`;
    }
}
