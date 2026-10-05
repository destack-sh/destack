import { join } from "node:path";
import { FileLock, runtimeDirectory } from "@destack/fs";
import type { Keychain } from "../keychain/keychain.ts";

/** The operating system's keychain, for one service's secrets. */
export class SystemKeychain implements Keychain {
    /** The keychain service, such as `app.destack.host`. */
    readonly service: string;
    /** The environment naming the runtime directory with the service's lock file. */
    readonly #environment: Readonly<Record<string, string | undefined>>;

    /** Open a keychain service. */
    constructor(
        service: string,
        environment: Readonly<Record<string, string | undefined>> = process.env,
    ) {
        this.service = service;
        this.#environment = environment;
    }

    /** Read a secret, absent before one was kept. */
    async load(name: string): Promise<string | undefined> {
        return (await Bun.secrets.get({ service: this.service, name })) ?? undefined;
    }

    /** Keep a secret, replacing the one kept before. */
    async save(name: string, value: string): Promise<void> {
        await using _lock = await this.#lock();
        await Bun.secrets.set({ service: this.service, name, value });
    }

    /** Keep a secret if the kept one equals the expected one. */
    async saveIf(name: string, expected: string | undefined, value: string): Promise<boolean> {
        // compare and keep under the service's lock
        await using _lock = await this.#lock();
        const isExpected = (await this.load(name)) === expected;
        if (isExpected) {
            await Bun.secrets.set({ service: this.service, name, value });
        }

        return isExpected;
    }

    /** Forget a secret. */
    async remove(name: string): Promise<void> {
        await using _lock = await this.#lock();
        await Bun.secrets.delete({ service: this.service, name });
    }

    /** Forget a secret if the kept one equals the expected one. */
    async removeIf(name: string, expected: string): Promise<boolean> {
        // compare and forget under the service's lock
        await using _lock = await this.#lock();
        const isExpected = (await this.load(name)) === expected;
        if (isExpected) {
            await Bun.secrets.delete({ service: this.service, name });
        }

        return isExpected;
    }

    /** Lock the service's writes across processes. */
    async #lock(): Promise<FileLock> {
        const directory = await runtimeDirectory({ environment: this.#environment });

        return FileLock.acquire(join(directory, `${this.service}.lock`));
    }
}
