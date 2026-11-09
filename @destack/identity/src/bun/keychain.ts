import { dirname, join } from "node:path";
import { type FileLock, type FileSystem, FileSystemError } from "@destack/environment";
import { schema } from "@destack/schema";
import type { Keychain } from "../keychain/index.ts";

/** The permission bits of a file keychain's files, their owner's alone, as with ssh keys. */
const PRIVATE_MODE = 0o600;

/** The permission bits of a file keychain's directory: its owner alone enters it. */
const PRIVATE_DIRECTORY_MODE = 0o700;

/** Where a keychain stores its service's secrets: the system's keychain, or a private file. */
export type KeychainStore =
    | {
          /** The system's keychain: Keychain, Credential Manager or the Secret Service. */
          readonly kind: "system";
      }
    | {
          /** A file per service, for a computer without one, such as a server. */
          readonly kind: "file";
          /** The directory with the files. */
          readonly directory: string;
      };

/** The secrets of one service a file keychain stores, by name. */
const Secrets = schema.record(schema.string(), schema.string());

/** A keychain of one service's secrets on this computer. */
export class BunKeychain implements Keychain {
    /** The keychain service, such as `app.destack.host`. */
    readonly service: string;
    /** Where the service's secrets lie. */
    readonly store: KeychainStore;
    /** The files with the runtime directory of the service's lock files. */
    readonly #files: FileSystem;

    /** Open a keychain service in a store, locking it through the files' runtime directory. */
    constructor(service: string, files: FileSystem, store: KeychainStore) {
        this.service = service;
        this.store = store;
        this.#files = files;
    }

    /** Read a secret, absent before one was stored. */
    async load(name: string): Promise<string | undefined> {
        // read the operating system's keychain
        if (this.store.kind === "system") {
            return (await Bun.secrets.get({ service: this.service, name })) ?? undefined;
        }
        // read the service's file
        else {
            return (await BunKeychain.#read(this.#path(this.store.directory), this.#files))[name];
        }
    }

    /** Store a secret, replacing the one stored before. */
    async save(name: string, value: string): Promise<void> {
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        await this.#set(name, value);
    }

    /** Store a secret if the stored one equals the expected one. */
    async saveIf(name: string, expected: string | undefined, value: string): Promise<boolean> {
        // compare and store under the service's lock
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        const isExpected = (await this.load(name)) === expected;
        if (isExpected) {
            await this.#set(name, value);
        }

        return isExpected;
    }

    /** Forget a secret. */
    async remove(name: string): Promise<void> {
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        await this.#delete(name);
    }

    /** Forget a secret if the stored one equals the expected one. */
    async removeIf(name: string, expected: string): Promise<boolean> {
        // compare and forget under the service's lock
        await using _lock = await this.#lockFile(`${this.service}.lock`);
        const isExpected = (await this.load(name)) === expected;
        if (isExpected) {
            await this.#delete(name);
        }

        return isExpected;
    }

    /** Lock a name exclusively across processes until disposed. */
    async lock(name: string): Promise<AsyncDisposable> {
        return this.#lockFile(`${this.service}.${name}.lock`);
    }

    /** Store a secret in the store, the caller holding the service's lock. */
    async #set(name: string, value: string): Promise<void> {
        // store in the operating system's keychain
        if (this.store.kind === "system") {
            await Bun.secrets.set({ service: this.service, name, value });
        }
        // store in the service's file
        else {
            const path = this.#path(this.store.directory);
            const secrets = await BunKeychain.#read(path, this.#files);
            await BunKeychain.#write(path, { ...secrets, [name]: value }, this.#files);
        }
    }

    /** Forget a secret in the store, the caller holding the service's lock. */
    async #delete(name: string): Promise<void> {
        // forget in the operating system's keychain
        if (this.store.kind === "system") {
            await Bun.secrets.delete({ service: this.service, name });
        }
        // forget in the service's file
        else {
            const path = this.#path(this.store.directory);
            const { [name]: _forgotten, ...kept } = await BunKeychain.#read(path, this.#files);
            await BunKeychain.#write(path, kept, this.#files);
        }
    }

    /** The file of the service's secrets in a directory. */
    #path(directory: string): string {
        return join(directory, `${this.service}.json`);
    }

    /** Read a service's file, empty before its first secret. */
    static async #read(path: string, files: FileSystem): Promise<Readonly<Record<string, string>>> {
        const text = await files
            .readTextFile(path)
            .catch((error: unknown) => FileSystemError.absent(error));

        return text === undefined ? {} : Secrets.parse(JSON.parse(text));
    }

    /** Replace a service's file whole, private from its creation, under the service's lock. */
    static async #write(
        path: string,
        secrets: Readonly<Record<string, string>>,
        files: FileSystem,
    ): Promise<void> {
        // stage the file beside it, then move it over, so a reader never sees half of it
        const staged = `${path}.staging`;
        await files.mkdir(dirname(path), { recursive: true, mode: PRIVATE_DIRECTORY_MODE });
        await files.writeTextFile(staged, JSON.stringify(secrets), { mode: PRIVATE_MODE });
        await files.rename(staged, path);
    }

    /** Lock a file in the runtime directory exclusively across processes. */
    async #lockFile(file: string): Promise<FileLock> {
        const directory = await this.#files.runtimeDirectory();

        return this.#files.lock(join(directory, file), "exclusive");
    }
}
