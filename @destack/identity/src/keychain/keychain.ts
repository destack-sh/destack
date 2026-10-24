/** Where a host stores its secrets by name, such as its machine key and its root keys. */
export interface Keychain {
    /** Read a secret, absent before one was stored. */
    readonly load: (name: string) => Promise<string | undefined>;
    /** Store a secret, replacing the one stored before. */
    readonly save: (name: string, value: string) => Promise<void>;
    /** Store a secret if the stored one equals the expected one, and report whether it did. */
    readonly saveIf: (
        name: string,
        expected: string | undefined,
        value: string,
    ) => Promise<boolean>;
    /** Forget a secret. */
    readonly remove: (name: string) => Promise<void>;
    /** Forget a secret if the stored one equals the expected one, and report whether it did. */
    readonly removeIf: (name: string, expected: string) => Promise<boolean>;
    /** Lock a name exclusively until disposed. */
    readonly lock: (name: string) => Promise<AsyncDisposable>;
}

/** The operations every keychain shares. */
export const Keychain = {
    update,
};

/** Change the secret stored under a name, retrying over a racing writer's secret. */
async function update(
    keychain: Keychain,
    name: string,
    change: (secret: string | undefined) => string | Promise<string>,
): Promise<string> {
    // retry a change that loses a race over the winner's secret
    while (true) {
        // write nothing for an unchanged secret
        const secret = await keychain.load(name);
        const changed = await change(secret);
        if (changed === secret) {
            return changed;
        }

        // store the change unless another writer changed the secret first
        if (await keychain.saveIf(name, secret, changed)) {
            return changed;
        }
    }
}

/** A keychain in memory, lost with the process. */
export class MemoryKeychain implements Keychain {
    /** The stored secrets, by name. */
    readonly secrets: Map<string, string>;
    /** The last lock of each name, which the next one waits for. */
    readonly #locks = new Map<string, Promise<void>>();

    /** Keep secrets in memory, starting from kept ones by name. */
    constructor(secrets: Readonly<Record<string, string>> = {}) {
        this.secrets = new Map(Object.entries(secrets));
    }

    /** Lock a name exclusively until disposed, after its earlier locks. */
    async lock(name: string): Promise<AsyncDisposable> {
        // queue behind the name's last lock
        const previous = this.#locks.get(name) ?? Promise.resolve();
        const released = Promise.withResolvers<void>();
        const locked = previous.then(() => released.promise);
        this.#locks.set(name, locked);
        await previous;

        // release the lock on disposal, forgetting it once last
        return {
            [Symbol.asyncDispose]: async () => {
                released.resolve();
                if (this.#locks.get(name) === locked) {
                    this.#locks.delete(name);
                }
            },
        };
    }

    /** Read a secret. */
    async load(name: string): Promise<string | undefined> {
        return this.secrets.get(name);
    }

    /** Store a secret. */
    async save(name: string, value: string): Promise<void> {
        this.secrets.set(name, value);
    }

    /** Store a secret if the stored one equals the expected one. */
    async saveIf(name: string, expected: string | undefined, value: string): Promise<boolean> {
        // compare and store without yielding in between
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

    /** Forget a secret if the stored one equals the expected one. */
    async removeIf(name: string, expected: string): Promise<boolean> {
        // compare and forget without yielding in between
        const isExpected = this.secrets.get(name) === expected;
        if (isExpected) {
            this.secrets.delete(name);
        }

        return isExpected;
    }
}
