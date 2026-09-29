import type { DatabaseConnection } from "@destack/db";
import type { Directory, ObjectClaims } from "@destack/directory";
import { ObjectType } from "../object/object.ts";

/** The names one request's write claims, reserved in the directory until the write commits. */
export class Reservation {
    /** The directory holding the claims. */
    readonly directory: Directory;
    /** The request whose write reserved them. */
    readonly requestId: string;
    /** The names each written object claims. */
    readonly owned: readonly ObjectClaims[];

    /** Hold a request's reservation. */
    private constructor(directory: Directory, requestId: string, owned: readonly ObjectClaims[]) {
        this.directory = directory;
        this.requestId = requestId;
        this.owned = owned;
    }

    /** Reserve the names that an open transaction's indexed rows claim and refuse names of other objects. */
    static async open(
        directory: Directory,
        transaction: DatabaseConnection,
        objects: readonly ObjectType[],
        requestId: string,
        now: number,
    ): Promise<Reservation> {
        // reserve every written object's claims
        const owned = await ObjectType.written(transaction, objects);
        const claims = owned.flatMap((entry) => entry.claims);
        if (claims.length > 0) {
            await directory.claim(claims, requestId, now);
        }

        return new Reservation(directory, requestId, owned);
    }

    /** Confirm the reservation once its write committed, releasing the names its objects dropped. */
    confirm(): Promise<void> {
        return this.directory.confirm(this.requestId, this.owned);
    }
}
