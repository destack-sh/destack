import { type ObjectReference } from "@destack/sync";
import { and, eq, isNull, type DatabaseConnection } from "@destack/db";
import {
    DirectoryCache,
    DirectoryDatabase,
    type Directory,
    type Location,
} from "@destack/directory";
import { DatabaseError } from "@destack/db/error";
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

    /** Resolve through a directory and a lookup of accounts by handle. */
    constructor(
        directory: Directory,
        account: (handle: string) => Promise<Identifier<"account"> | undefined>,
    ) {
        this.directory = directory;
        this.#lookup = account;
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

    /** Find the account with a handle, absent once its deletion was requested. */
    account(handle: string): Promise<Identifier<"account"> | undefined> {
        return this.#lookup(handle);
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
    async resolve(type: ObjectType, address: string): Promise<Location> {
        // find the object, then its zone and cell
        const named = await this.find(type, address);
        const zone = named === undefined ? undefined : await this.directory.locate(named.id);
        const cell = zone === undefined ? undefined : await this.directory.cell(zone.cell);
        if (zone === undefined || cell === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `nothing answers at ${address}` });
        }

        return { ...zone, endpoint: cell.endpoint };
    }
}

/** The global tier's resolver that keeps its reads until their rows change. */
export class ResolverCache extends Resolver {
    /** The global database the reads come from. */
    readonly database: DatabaseConnection;
    /** The directory's cached reads. */
    override readonly directory: DirectoryCache;
    /** Accounts by handle. */
    readonly #accounts = new ReadCache<Identifier<"account"> | undefined>(HANDLE_CAPACITY);

    /** Resolve through the global database and keep each read. */
    constructor(database: DatabaseConnection) {
        // cache zone and handle reads over the global database
        const directory = new DirectoryCache(new DirectoryDatabase(database));
        super(directory, (handle) => Resolver.account(database, handle));
        this.database = database;
        this.directory = directory;
    }

    /** Find the account with a handle, once until its account changes. */
    override account(handle: string): Promise<Identifier<"account"> | undefined> {
        return this.#accounts.get(handle, () => super.account(handle));
    }

    /** Forget the reads each logged change affects, following the log until the signal aborts. */
    async follow(signal: AbortSignal): Promise<void> {
        await Promise.all([this.directory.follow(signal), this.#followAccounts(signal)]);
    }

    /** Forget the handles of changed accounts until the signal aborts. */
    async #followAccounts(signal: AbortSignal): Promise<void> {
        const log = this.database.log;
        while (!signal.aborted) {
            // forget every handle, and follow the account changes committed since
            const after = (await log.position()).sequence;
            this.#accounts.clear();
            try {
                for await (const page of log.follow({ tables: [account.table], after }, signal)) {
                    for (const change of page.changes) {
                        for (const image of [change.before, change.after]) {
                            if (image !== undefined) {
                                this.#accounts.forget(AccountHandle.parse(image.handle));
                            }
                        }
                    }
                }
            } catch (error) {
                // start over once the changes were compacted away
                if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                    throw error;
                }
            }
        }
    }
}
