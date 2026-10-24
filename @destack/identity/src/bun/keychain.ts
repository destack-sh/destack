import { join } from "node:path";
import type { FileLock, FileSystem } from "@destack/environment";
import type { Keychain } from "../keychain/index.ts";

/** The operating system's keychain, for one service's secrets. */
export class BunKeychain implements Keychain {
    /** The keychain service, such as `app.destack.host`. */
    readonly service: string;
    /** The files with the runtime directory of the service's lock files. */
    readonly #files: FileSystem;

    /** Open a keychain service, locking it through the files' runtime directory. */
    constructor(service: string, files: FileSystem) {
        this.service = service;
        this.#files = files;
    }

    /** Read a secret, absent before one was stored. */
    async load(name: string): Promise<string | undefined> {
        return (await Bun.secrets.get({ service: this.service, name })) ?? undefined;
    }

    /** Store a secret, replacing the one stored before. */
    async save(name: string, value: string): Promise<void> {
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        await Bun.secrets.set({ service: this.service, name, value });
    }

    /** Store a secret if the stored one equals the expected one. */
    async saveIf(name: string, expected: string | undefined, value: string): Promise<boolean> {
        // compare and store under the service's lock
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        const isExpected = (await this.load(name)) === expected;
        if (isExpected) {
            await Bun.secrets.set({ service: this.service, name, value });
        }

        return isExpected;
    }

    /** Forget a secret. */
    async remove(name: string): Promise<void> {
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        await Bun.secrets.delete({ service: this.service, name });
    }

    /** Forget a secret if the stored one equals the expected one. */
    async removeIf(name: string, expected: string): Promise<boolean> {
        // compare and forget under the service's lock
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        const isExpected = (await this.load(name)) === expected;
        if (isExpected) {
            await Bun.secrets.delete({ service: this.service, name });
        }

        return isExpected;
    }

    /** Lock a name exclusively across processes until disposed. */
    async lock(name: string): Promise<AsyncDisposable> {
        return this.#lockFile(`${this.service}.${name}.lock`);
    }

    /** Lock a file in the runtime directory exclusively across processes. */
    async #lockFile(file: string): Promise<FileLock> {
        const directory = await this.#files.runtimeDirectory();

        return this.#files.lock(join(directory, file), "exclusive");
    }
}
