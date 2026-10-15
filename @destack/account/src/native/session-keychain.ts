import { Keychain } from "@destack/identity";
import type { Person } from "./person.ts";

/** The keychain service keeping the refresh tokens of people signed in on a native client. */
export const SESSIONS = "app.destack.login";

/** The refresh tokens of the people signed in on this computer, one keychain entry each. */
export class SessionKeychain {
    /** The keychain keeping the sessions. */
    readonly keychain: Keychain;

    /** Keep sessions in a keychain. */
    constructor(keychain: Keychain) {
        this.keychain = keychain;
    }

    /** Read a person's session, absent after sign-out. */
    load(person: Person): Promise<string | undefined> {
        return this.keychain.load(SessionKeychain.#entry(person));
    }

    /** Keep a person's session in place of the one read before. */
    async keep(person: Person, token: string): Promise<string> {
        // replace the session read before or adopt the racing sign-in's session
        const entry = SessionKeychain.#entry(person);
        const current = await this.keychain.load(entry);

        return Keychain.update(this.keychain, entry, (existing) =>
            existing === current || existing === undefined ? token : existing,
        );
    }

    /** Forget a person's session, whichever one the keychain keeps. */
    forget(person: Person): Promise<void> {
        return this.keychain.remove(SessionKeychain.#entry(person));
    }

    /** Forget a person's session while it is the one read before, and report whether it did. */
    remove(person: Person, token: string): Promise<boolean> {
        return this.keychain.removeIf(SessionKeychain.#entry(person), token);
    }

    /** Find a person's keychain entry by the full issuer. */
    static #entry(person: Person): string {
        return `${person.subject}@${person.issuer}`;
    }
}
