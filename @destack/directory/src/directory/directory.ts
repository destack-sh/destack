import type { Claim, Expiry, ObjectClaims } from "../claim/claim.ts";
import type { Endpoint, Placement } from "../placement/placement.ts";
import type { Identity } from "@destack/identity";

/** The universe's directory: which machine serves each space, where each machine answers, each identity's signed operations, and which object claims each unique name. */
export interface Directory {
    // placements and machines

    /** Place a space on its machine: create the placement, keep it and end its move, advance its epoch on its own machine, or take it over on its move's target at the next epoch. */
    place(placement: Placement): Promise<void>;

    /** Withdraw a space its machine serves at an epoch. */
    withdraw(placement: Placement): Promise<void>;

    /** Find where a space runs. */
    locate(space: string): Promise<Placement | undefined>;

    /** List the placements of an account's spaces. */
    list(account: string): Promise<readonly Placement[]>;

    /** Mark a space its machine serves at an epoch as moving to a target machine. */
    move(placement: Placement, target: string): Promise<void>;

    /** Give a machine work in a space, as the machine serving the space at its epoch. */
    assign(placement: Placement, machine: string): Promise<void>;

    /** Withdraw a machine's work in a space, as the machine serving the space at its epoch. */
    unassign(placement: Placement, machine: string): Promise<void>;

    /** List the spaces that gave a machine work, in identity order. */
    assignments(machine: string): Promise<readonly string[]>;

    /** List the machines a space gave work, in identity order. */
    assigned(space: string): Promise<readonly string[]>;

    /** Record the URL a machine answers at under a publication: a token per tunnel or serving process. */
    publish(machine: string, scope: string, url: string, publication: string): Promise<void>;

    /** Withdraw the URL a machine answers at while the publication named still holds it, as a machine's tunnel closes. */
    unpublish(machine: string, publication: string): Promise<void>;

    /** Read the URL a machine answers at, absent before it published one. */
    endpoint(machine: string): Promise<Endpoint | undefined>;

    // identities

    /** Apply a signed identity operation: a space's first only from the machine serving it, the universe's first from the process hosting the directory. */
    apply(operation: string, placement?: Placement): Promise<void>;

    /** Read a space's or the universe's current identity, absent before its first operation. */
    identity(subject: string): Promise<Identity | undefined>;

    /** List an identity's signed operations in order, nullified ones included, for auditing. */
    operations(subject: string): Promise<readonly string[]>;

    // claims

    /** Reserve a request's claims until its write commits, returning the names other objects own. */
    claim(claims: readonly Claim[], requestId: string): Promise<readonly Claim[]>;

    /** Settle the names of a request's committed write in its objects' states and release the names its objects dropped. */
    confirm(requestId: string, owned: readonly ObjectClaims[]): Promise<void>;

    /** Release the reservations of a request with a failed write. */
    release(requestId: string): Promise<void>;

    /** Replace an object's claims after a write without reservations, unless other objects own some of its names, which it returns. */
    replace(owned: ObjectClaims, requestId: string): Promise<readonly Claim[]>;

    /** Find the object an active name resolves to, none for a reserved name. */
    owner(index: string, key: string): Promise<Pick<Claim, "objectId" | "scope"> | undefined>;

    /** List the expired reservations of some indexes, and the next deadline, by the directory's clock. */
    expired(indexes: readonly string[]): Promise<Expiry>;
}
