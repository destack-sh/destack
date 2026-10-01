import { type ObjectReference } from "@destack/sync";
import { and, eq, isNull, type DatabaseConnection } from "@destack/db";
import { DirectoryStore, type Directory, type Cell, type Zone } from "@destack/directory";
import { ReadCache } from "@destack/db/log";
import type { ObjectType } from "@destack/object";
import type { Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { account } from "../object/account.ts";
import { AccountHandle } from "../object/handle.ts";

/** The most handles a resolver cache keeps: about 3 MiB at 200 bytes an entry. */
const HANDLE_CAPACITY = 16_384;

/** The resolver of addresses `<name>.<handle>`, an object's name in the account with the handle. */
export class Resolver {
    /** The directory holding the objects' names, zones and cells. */
    readonly directory: Directory;
    /** Find the account with a handle. */
    readonly #lookup: (handle: string) => Promise<Identifier<"account"> | undefined>;
    /** Accounts by handle, kept while the global database's log is followed. */
    readonly #accounts = new ReadCache<Identifier<"account"> | undefined>(HANDLE_CAPACITY);
    /** The global database and its directory, for a resolver of the global tier. */
    #global: { readonly database: DatabaseConnection; readonly store: DirectoryStore } | undefined;
    /** Whether the global database's log is followed, which keeps the handle reads current. */
    #isFollowing = false;

    /** Resolve through a directory and a lookup of accounts by handle. */
    constructor(
        directory: Directory,
        account: (handle: string) => Promise<Identifier<"account"> | undefined>,
    ) {
        this.directory = directory;
        this.#lookup = account;
    }

    /** Resolve through the global database, keeping reads while its log is followed. */
    static global(database: DatabaseConnection): Resolver {
        // resolve through the global database's directory and accounts
        const store = new DirectoryStore(database);
        const resolver = new Resolver(store, (handle) => Resolver.account(database, handle));
        resolver.#global = { database, store };

        return resolver;
    }

    /** Find the account with a handle in the global database, absent once its deletion was requested. */
    static async account(
        database: DatabaseConnection,
        handle: string,
    ): Promise<Identifier<"account"> | undefined> {
        const [named] = await database
            .select({ id: account.table.id })
            .from(account.table)
            .where(
                and(eq(account.table.handle, handle), isNull(account.table.deletionRequestedAt)),
            );

        return named?.id;
    }

    /** Find the account with a handle, absent once its deletion was requested, once until its account changes while following. */
    account(handle: string): Promise<Identifier<"account"> | undefined> {
        return this.#isFollowing
            ? this.#accounts.get(handle, () => this.#lookup(handle))
            : this.#lookup(handle);
    }

    /** Keep the directory's and the handles' reads, forgetting those each logged change affects, until the signal aborts. */
    async follow(signal: AbortSignal): Promise<void> {
        // refuse following a resolver outside the global tier
        const global = this.#global;
        if (global === undefined) {
            throw new TypeError("only a resolver over the global database follows its log");
        }

        // follow the directory and the accounts, keeping handle reads meanwhile
        this.#isFollowing = true;
        try {
            await Promise.all([
                global.store.follow(signal),
                this.#followAccounts(global.database, signal),
            ]);
        } finally {
            this.#isFollowing = false;
            this.#accounts.clear();
        }
    }

    /** Find the object at an address among an account's objects of a type, absent for none. */
    async find(type: ObjectType, address: string): Promise<ObjectReference | undefined> {
        // find the account with the handle
        const dot = address.lastIndexOf(".");
        const handle = AccountHandle.safeParse(address.slice(dot + 1));
        const accountId = dot > 0 && handle.success ? await this.account(handle.data) : undefined;

        // find the object claiming the name within the account
        return accountId === undefined
            ? undefined
            : type.lookup(this.directory, "name", [address.slice(0, dot)], accountId);
    }

    /** Resolve an address of an object type to its zone and the URL its cell answers at. */
    async resolve(type: ObjectType, address: string): Promise<Zone & Pick<Cell, "endpoint">> {
        // find the object, then its zone and cell
        const named = await this.find(type, address);
        const zone = named === undefined ? undefined : await this.directory.locate(named.id);
        const cell = zone === undefined ? undefined : await this.directory.cell(zone.cell);
        if (zone === undefined || cell === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `nothing answers at ${address}` });
        }

        return { ...zone, endpoint: cell.endpoint };
    }

    /** Forget the handles of changed accounts until the signal aborts. */
    #followAccounts(database: DatabaseConnection, signal: AbortSignal): Promise<void> {
        return database.log.invalidate(
            [account.table],
            {
                clear: () => this.#accounts.clear(),
                forget: (change) => {
                    for (const image of [change.before, change.after]) {
                        if (image !== undefined) {
                            this.#accounts.forget(AccountHandle.parse(image.handle));
                        }
                    }
                },
            },
            signal,
        );
    }
}
