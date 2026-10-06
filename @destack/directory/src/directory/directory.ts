import { ServiceMount, type Service, type ServiceRouter } from "@destack/service";
import { createClient } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";
import type { Claim, Expiry, ObjectClaims } from "../claim/claim.ts";
import { Moved } from "../moved/moved.ts";
import type { Cell, Zone } from "../zone/zone.ts";
import { type Identity, PublicKey } from "@destack/identity";
import { createLocalJWKSet, errors, type JWTVerifyGetKey } from "jose";

/** How long a verifier keeps an identity's key set before reading it again, which a new signing key waits out before signing: ten minutes. */
export const KEY_SET_MILLISECONDS = 10 * 60_000;

/** The most identities' key sets a verifier keeps, the oldest read dropping first. */
const KEY_SETS = 16_384;

/** How long a verifier waits before reading a key set again for a key it lacks: thirty seconds. */
const KEY_SET_COOLDOWN_MILLISECONDS = 30_000;

/** An identity's key set as a verifier keeps it, with when it was read. */
interface KeptKeySet {
    /** The key set's getter. */
    readonly keys: JWTVerifyGetKey;
    /** When it was read, in UTC epoch milliseconds. */
    readonly readAt: number;
}

/** The universe's cell router: which cell serves each zone, where each cell answers, and which object claims each unique name. */
export abstract class Directory {
    /** The key sets verifiers read, by subject. */
    readonly #keySets = new Map<string, Promise<KeptKeySet>>();

    // zones and cells

    /** Place a zone in its cell: create it, keep its placement and end its move, advance its epoch in its own cell, or take it over in its move's target at the next epoch. */
    abstract place(zone: Zone): Promise<void>;

    /** Withdraw a zone its cell serves at an epoch. */
    abstract withdraw(zone: Zone): Promise<void>;

    /** Find the zone of a scope's databases. */
    abstract locate(scope: string): Promise<Zone | undefined>;

    /** List the zones a scope contains, such as an account's spaces. */
    abstract list(scope: string): Promise<readonly Zone[]>;

    /** Mark a zone its cell serves at an epoch as moving to a target cell. */
    abstract move(zone: Zone, target: string): Promise<void>;

    /** Give a cell work in a zone, as the cell serving the zone at its epoch. */
    abstract assign(zone: Zone, cell: string): Promise<void>;

    /** Withdraw a cell's work in a zone, as the cell serving the zone at its epoch. */
    abstract unassign(zone: Zone, cell: string): Promise<void>;

    /** List the zones that gave a cell work, in identity order. */
    abstract assignments(cell: string): Promise<readonly string[]>;

    /** List the cells a zone gave work, in identity order. */
    abstract assigned(zone: string): Promise<readonly string[]>;

    /** Record the URL a cell answers at. */
    abstract publish(cell: string, scope: string, endpoint: string): Promise<void>;

    /** Read a cell, absent before it published an endpoint. */
    abstract cell(id: string): Promise<Cell | undefined>;

    // identities

    /** Apply a signed identity operation: a space's first only from the cell serving its zone, the universe's first from the process hosting the directory. */
    abstract apply(operation: string, zone?: Zone): Promise<void>;

    /** Read a space's or the universe's current identity, absent before its first operation. */
    abstract identity(subject: string): Promise<Identity | undefined>;

    /** List an identity's signed operations in order, nullified ones included, for auditing. */
    abstract operations(subject: string): Promise<readonly string[]>;

    /** Read the keys verifying what an identity signs, kept for ten minutes and read again for a key it lacks, as a remote JSON Web Key Set is. */
    keys(subject: string): JWTVerifyGetKey {
        return async (header, token) => {
            // read the key set once its keeping lapsed
            const now = this.now();
            let kept = await this.#keySet(subject, now, KEY_SET_MILLISECONDS);
            try {
                return await kept.keys(header, token);
            } catch (error) {
                // read it again for a key it lacks, once the cooldown passed
                if (!(error instanceof errors.JWKSNoMatchingKey)) {
                    throw error;
                }
                kept = await this.#keySet(subject, now, KEY_SET_COOLDOWN_MILLISECONDS);

                return kept.keys(header, token);
            }
        };
    }

    /** Read the directory's clock, in UTC epoch milliseconds. */
    protected now(): number {
        return Date.now();
    }

    /** Read an identity's key set, kept unless it is older than an age. */
    #keySet(subject: string, now: number, age: number): Promise<KeptKeySet> {
        // keep a key set read within the age, and forget a failed read
        const kept = this.#keySets.get(subject);
        const reading =
            kept === undefined
                ? this.#readKeySet(subject, now)
                : kept.then((keySet) =>
                      now - keySet.readAt < age ? keySet : this.#readKeySet(subject, now),
                  );
        this.#keySets.delete(subject);
        this.#keySets.set(subject, reading);
        reading.catch(() => this.#keySets.delete(subject));
        for (const oldest of this.#keySets.keys()) {
            if (this.#keySets.size <= KEY_SETS) {
                break;
            }
            this.#keySets.delete(oldest);
        }

        return reading;
    }

    /** Read an identity's key set from its current state, refusing a subject without an identity. */
    async #readKeySet(subject: string, now: number): Promise<KeptKeySet> {
        const identity = await this.identity(subject);
        if (identity === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: `${subject} has no identity` });
        }

        return {
            keys: createLocalJWKSet(await PublicKey.keySet(identity.signingKeys)),
            readAt: now,
        };
    }

    // claims

    /** Reserve a request's claims until its write commits, returning the names other objects own. */
    abstract claim(claims: readonly Claim[], requestId: string): Promise<readonly Claim[]>;

    /** Confirm a request's reserved claims and release the names its objects dropped. */
    abstract confirm(requestId: string, owned: readonly ObjectClaims[]): Promise<void>;

    /** Release the reservations of a request with a failed write. */
    abstract release(requestId: string): Promise<void>;

    /** Replace an object's claims after a write without reservations, unless other objects own some of its names, which it returns. */
    abstract replace(owned: ObjectClaims, requestId: string): Promise<readonly Claim[]>;

    /** Find the object owning a confirmed name. */
    abstract owner(
        index: string,
        key: string,
    ): Promise<Pick<Claim, "objectId" | "scope"> | undefined>;

    /** List the expired reservations of some indexes, and the next deadline, by the directory's clock. */
    abstract expired(indexes: readonly string[]): Promise<Expiry>;

    // clients

    /** Create a client of a service where a cell mounts it, following a moved scope to its new cell. */
    client<Router extends ServiceRouter>(
        service: Service<Router>,
        cell: string,
        fetch: (request: Request) => Promise<Response>,
    ) {
        // resolve the current cell's endpoint for every request
        let current = cell;
        const endpoint = async () => {
            const found = await this.cell(current);
            if (found === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `cell ${current} publishes no endpoint`,
                });
            }

            return `${found.endpoint.replace(/\/+$/u, "")}${ServiceMount.path(service.package.id)}`;
        };

        return createClient(service, {
            url: endpoint,
            fetch: async (request, options) => {
                // send the request and keep its body for the moved scope's cell
                const sent = new Request(request, options);
                const body = sent.body === null ? null : await sent.clone().arrayBuffer();
                const from = await endpoint();
                const response = await fetch(sent);
                const moved = await Moved.read(response);
                if (moved === undefined) {
                    return response;
                }

                // resend once to the cell the scope moved to
                current = moved.cell;
                const target = `${await endpoint()}${sent.url.slice(from.length)}`;

                return fetch(
                    new Request(target, { method: sent.method, headers: sent.headers, body }),
                );
            },
        });
    }
}
