import {
    Directory,
    type Cell,
    type Claim,
    type ClaimOwner,
    type Expiry,
    type ObjectClaims,
    type Zone,
} from "@destack/directory";
import type { Identifier } from "@destack/schema";
import { Resolver } from "../directory/resolver.ts";
import type { connect } from "./client.ts";

/** The universe's directory reached through the account service, as a host. */
export class DirectoryClient extends Directory {
    /** The account service client, authenticated as the host. */
    readonly #client: ReturnType<typeof connect>;

    /** Reach the directory through an authenticated account service client. */
    constructor(client: ReturnType<typeof connect>) {
        super();
        this.#client = client;
    }

    /** Place a zone in its cell at an epoch. */
    async place(zone: Zone): Promise<void> {
        await this.#client.directory.place({ zone });
    }

    /** Withdraw a zone its cell serves at an epoch. */
    async withdraw(zone: Zone): Promise<void> {
        await this.#client.directory.withdraw({ zone });
    }

    /** Find the zone of a scope's databases. */
    async locate(scope: string): Promise<Zone | undefined> {
        return (await this.#client.directory.locate({ scope })) ?? undefined;
    }

    /** List the zones a scope contains. */
    async list(scope: string): Promise<readonly Zone[]> {
        return this.#client.directory.list({ scope });
    }

    /** Mark a zone the host's cell serves as moving to a target cell. */
    async move(zone: Zone, target: string): Promise<void> {
        await this.#client.directory.move({ zone, target });
    }

    /** Record the URL a cell the host acts for answers at. */
    async publish(cell: string, scope: string, endpoint: string): Promise<void> {
        await this.#client.directory.publish({ cell, scope, endpoint });
    }

    /** Read a cell, absent before it published an endpoint. */
    async cell(id: string): Promise<Cell | undefined> {
        return (await this.#client.directory.cell({ cell: id })) ?? undefined;
    }

    /** Reserve a request's claims until its write commits. */
    async claim(claims: readonly Claim[], requestId: string): Promise<void> {
        await this.#client.directory.claim({ requestId, claims: [...claims] });
    }

    /** Confirm a request's reserved claims. */
    async confirm(requestId: string, owned: readonly ObjectClaims[]): Promise<void> {
        await this.#client.directory.confirm({ requestId, owned: owned.map(plain) });
    }

    /** Release the reservations of a request with a failed write. */
    async release(requestId: string): Promise<void> {
        await this.#client.directory.release({ requestId });
    }

    /** Replace an object's claims after a write that reserved none. */
    async replace(owned: ObjectClaims, requestId: string): Promise<void> {
        await this.#client.directory.replace({ requestId, owned: plain(owned) });
    }

    /** Find the object owning a confirmed name. */
    async owner(index: string, key: string): Promise<ClaimOwner | undefined> {
        return (await this.#client.directory.owner({ index, key })) ?? undefined;
    }

    /** List the expired reservations of some indexes. */
    async expired(indexes: readonly string[]): Promise<Expiry> {
        return this.#client.directory.expired({ indexes: [...indexes] });
    }

    /** Find the account with a handle. */
    async account(handle: string): Promise<Identifier<"account"> | undefined> {
        return (await this.#client.directory.account({ handle })) ?? undefined;
    }

    /** Resolve addresses through this directory and the account service's handles. */
    resolver(): Resolver {
        return new Resolver(this, (handle) => this.account(handle));
    }
}

/** Copy an object's claims into mutable arrays. */
function plain(owned: ObjectClaims) {
    return { ...owned, indexes: [...owned.indexes], claims: [...owned.claims] };
}
