import type { DatabaseConnection } from "@destack/db";
import { type Directory, DirectoryStore, type ObjectClaims } from "@destack/directory";
import { ObjectType } from "../object/object.ts";

/** The names one request's write claims, reserved in the directory until the write commits. */
export class Reservation {
    /** The directory with the claims. */
    readonly directory: Directory;
    /** The request whose write reserved them. */
    readonly requestId: string;
    /** The names each written object claims. */
    readonly owned: readonly ObjectClaims[];

    /** Keep a request's reservation. */
    private constructor(directory: Directory, requestId: string, owned: readonly ObjectClaims[]) {
        this.directory = directory;
        this.requestId = requestId;
        this.owned = owned;
    }

    /** Reserve the names that an open transaction's indexed rows claim and refuse names of other objects, or claim them in the transaction itself where the directory keeps its claims in the same database. */
    static async open(
        directory: Directory,
        transaction: DatabaseConnection,
        objects: readonly ObjectType[],
        requestId: string,
    ): Promise<Reservation | undefined> {
        // claim every written object's names in the transaction of a directory in the same database, none for a step writing no object
        const owned = await ObjectType.written(transaction, objects);
        if (owned.length === 0) {
            return undefined;
        } else if (
            directory instanceof DirectoryStore &&
            directory.database.state === transaction.state
        ) {
            const within = new DirectoryStore(transaction);
            for (const entry of owned) {
                ObjectType.refuse(objects, await within.replace(entry, requestId));
            }

            return undefined;
        }

        // reserve every written object's claims in a directory elsewhere
        const claims = owned.flatMap((entry) => entry.claims);
        if (claims.length > 0) {
            ObjectType.refuse(objects, await directory.claim(claims, requestId));
        }

        return new Reservation(directory, requestId, owned);
    }

    /** Confirm the reservation once its write committed, releasing the names its objects dropped. */
    confirm(): Promise<void> {
        return this.directory.confirm(this.requestId, this.owned);
    }
}
