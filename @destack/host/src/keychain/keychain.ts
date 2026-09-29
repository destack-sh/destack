/** Where a host keeps its secrets by name, such as its private key and its vault root keys. */
export interface Keychain {
    /** Read a secret, absent before one was kept. */
    load(name: string): Promise<string | undefined>;
    /** Keep a secret, replacing the one kept before. */
    save(name: string, value: string): Promise<void>;
    /** Forget a secret. */
    remove(name: string): Promise<void>;
}

/** A keychain in memory, lost with the process. */
export class MemoryKeychain implements Keychain {
    /** The kept secrets, by name. */
    readonly secrets = new Map<string, string>();

    /** Read a secret. */
    async load(name: string): Promise<string | undefined> {
        return this.secrets.get(name);
    }

    /** Keep a secret. */
    async save(name: string, value: string): Promise<void> {
        this.secrets.set(name, value);
    }

    /** Forget a secret. */
    async remove(name: string): Promise<void> {
        this.secrets.delete(name);
    }
}
