/** Where a host keeps its secrets by name, such as its private key and its vault root keys. */
export interface Keychain {
    /** Read a secret, absent before one was kept. */
    load(name: string): Promise<string | undefined>;
    /** Keep a secret, replacing the one kept before. */
    save(name: string, value: string): Promise<void>;
    /** Keep a secret if the kept one equals the expected one, and report whether it did. */
    saveIf(name: string, expected: string | undefined, value: string): Promise<boolean>;
    /** Forget a secret. */
    remove(name: string): Promise<void>;
    /** Forget a secret if the kept one equals the expected one, and report whether it did. */
    removeIf(name: string, expected: string): Promise<boolean>;
}

/** The operations every keychain shares. */
export const Keychain = {
    update,
};

/** Change the secret kept under a name, retrying over a racing writer's secret, and return the secret kept. */
async function update(
    keychain: Keychain,
    name: string,
    change: (kept: string | undefined) => string | Promise<string>,
): Promise<string> {
    // retry a change that loses a race over the winner's secret
    while (true) {
        // change the kept secret, and keep it unchanged without writing
        const kept = await keychain.load(name);
        const changed = await change(kept);
        if (changed === kept) {
            return changed;
        }

        // keep the change unless another writer changed the secret first
        if (await keychain.saveIf(name, kept, changed)) {
            return changed;
        }
    }
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

    /** Keep a secret if the kept one equals the expected one. */
    async saveIf(name: string, expected: string | undefined, value: string): Promise<boolean> {
        // compare and keep without yielding in between
        const isExpected = this.secrets.get(name) === expected;
        if (isExpected) {
            this.secrets.set(name, value);
        }

        return isExpected;
    }

    /** Forget a secret. */
    async remove(name: string): Promise<void> {
        this.secrets.delete(name);
    }

    /** Forget a secret if the kept one equals the expected one. */
    async removeIf(name: string, expected: string): Promise<boolean> {
        // compare and forget without yielding in between
        const isExpected = this.secrets.get(name) === expected;
        if (isExpected) {
            this.secrets.delete(name);
        }

        return isExpected;
    }
}
