import {
    InstallationMount,
    SpaceMount,
    ServiceMount,
    type Service,
    type ServiceRouter,
} from "@destack/service";
import { type Authentication, Bearer, TokenVerifier } from "@destack/service/authentication";
import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { createClient } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";
import type { Claim, Expiry, ObjectClaims } from "../claim/claim.ts";
import { Moved } from "../moved/moved.ts";
import type { Endpoint, Placement } from "../placement/placement.ts";
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

/** The universe's router: which machine serves each space, where each machine answers, and which object claims each unique name. */
export abstract class Directory {
    /** The key sets verifiers read, by subject. */
    readonly #keySets = new Map<string, Promise<KeptKeySet>>();

    // placements and machines

    /** Place a space on its machine: create the placement, keep it and end its move, advance its epoch on its own machine, or take it over on its move's target at the next epoch. */
    abstract place(placement: Placement): Promise<void>;

    /** Withdraw a space its machine serves at an epoch. */
    abstract withdraw(placement: Placement): Promise<void>;

    /** Find where a space runs. */
    abstract locate(space: string): Promise<Placement | undefined>;

    /** List the placements of an account's spaces. */
    abstract list(account: string): Promise<readonly Placement[]>;

    /** Mark a space its machine serves at an epoch as moving to a target machine. */
    abstract move(placement: Placement, target: string): Promise<void>;

    /** Give a machine work in a space, as the machine serving the space at its epoch. */
    abstract assign(placement: Placement, machine: string): Promise<void>;

    /** Withdraw a machine's work in a space, as the machine serving the space at its epoch. */
    abstract unassign(placement: Placement, machine: string): Promise<void>;

    /** List the spaces that gave a machine work, in identity order. */
    abstract assignments(machine: string): Promise<readonly string[]>;

    /** List the machines a space gave work, in identity order. */
    abstract assigned(space: string): Promise<readonly string[]>;

    /** Record the URL a machine answers at under a publication: a token per tunnel or serving process. */
    abstract publish(
        machine: string,
        scope: string,
        url: string,
        publication: string,
    ): Promise<void>;

    /** Withdraw the URL a machine answers at while the publication named still holds it, as a machine's tunnel closes. */
    abstract unpublish(machine: string, publication: string): Promise<void>;

    /** Read the URL a machine answers at, absent before it published one. */
    abstract endpoint(machine: string): Promise<Endpoint | undefined>;

    // identities

    /** Apply a signed identity operation: a space's first only from the machine serving it, the universe's first from the process hosting the directory. */
    abstract apply(operation: string, placement?: Placement): Promise<void>;

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

    /** Verify a request's bearer token by its issuer's keys. */
    authenticate(
        request: Request,
        options: Parameters<Directory["verify"]>[1],
        now = Date.now(),
    ): Promise<Authentication> {
        return this.verify(Bearer.require(request.headers), options, now);
    }

    /** Verify a token by its issuer's keys: the universe's for a token the universe issued, else the space's that signed it, refusing one bound to another space than the call targets. */
    async verify(
        token: string,
        options: {
            /** The package receiving the call. */
            readonly audience: PackageId;
            /** The universe's issuer, which its tokens name, absent to accept tokens of spaces alone. */
            readonly universe?: string;
            /** The space the call targets, absent for a call outside any one space. */
            readonly scope?: string;
        },
        now = Date.now(),
    ): Promise<Authentication> {
        // read the issuing space, or the universe
        const issuer = Bearer.issuer(token);
        const space = schema.identifier("space").safeParse(issuer);
        let verifier: TokenVerifier;
        if (options.universe !== undefined && issuer === options.universe) {
            verifier = new TokenVerifier({
                authority: { kind: "universe" },
                issuer: options.universe,
                audience: options.audience,
                keys: this.keys(Scope.universe.id),
            });
        } else if (space.success) {
            verifier = new TokenVerifier({
                authority: { kind: "space", space: space.data },
                issuer: space.data,
                audience: options.audience,
                keys: this.keys(space.data),
            });
        } else {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid access token" });
        }

        return verifier.verify(token, options.scope, now);
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

    /** Settle the names of a request's committed write in its objects' states and release the names its objects dropped. */
    abstract confirm(requestId: string, owned: readonly ObjectClaims[]): Promise<void>;

    /** Release the reservations of a request with a failed write. */
    abstract release(requestId: string): Promise<void>;

    /** Replace an object's claims after a write without reservations, unless other objects own some of its names, which it returns. */
    abstract replace(owned: ObjectClaims, requestId: string): Promise<readonly Claim[]>;

    /** Find the object an active name resolves to, none for a reserved name. */
    abstract owner(
        index: string,
        key: string,
    ): Promise<Pick<Claim, "objectId" | "scope"> | undefined>;

    /** List the expired reservations of some indexes, and the next deadline, by the directory's clock. */
    abstract expired(indexes: readonly string[]): Promise<Expiry>;

    // clients

    /** Create a client of a service in a space, below the space's path where the machine serving it mounts the service, following the space as it moves. */
    spaceClient<Router extends ServiceRouter>(
        service: Service<Router>,
        space: string,
        fetch: (request: Request) => Promise<Response>,
    ) {
        return createClient(service, {
            url: async () =>
                `${await this.spaceUrl(space)}${ServiceMount.path(service.package.id)}`,
            fetch,
        });
    }

    /** Find the URL below which a space's machine serves its services. */
    async spaceUrl(space: string): Promise<string> {
        return SpaceMount.url(await this.#served(space), space);
    }

    /** Create a client of a service of an installation, by the installation's identifier or alias, below the installation's path where it mounts the service, following the space as it moves. */
    installationClient<Router extends ServiceRouter>(
        service: Service<Router>,
        space: string,
        installation: string,
        fetch: (request: Request) => Promise<Response>,
    ) {
        return createClient(service, {
            url: async () =>
                `${await this.installationUrl(space, installation)}${ServiceMount.path(service.package.id)}`,
            fetch,
        });
    }

    /** Find the URL below which an installation, by its identifier or alias, mounts its services on the machine serving its space. */
    async installationUrl(space: string, installation: string): Promise<string> {
        return InstallationMount.url(await this.#served(space), space, installation);
    }

    /** Find the endpoint URL of the machine serving a space, refusing a space no machine serves. */
    async #served(space: string): Promise<string> {
        // find the space's machine and its endpoint
        const placed = await this.locate(space);
        const found = placed === undefined ? undefined : await this.endpoint(placed.machine);
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no machine serves ${space}` });
        }

        return found.url;
    }

    /** Create a client of a service where a machine mounts it, following a moved scope to its new machine. */
    machineClient<Router extends ServiceRouter>(
        service: Service<Router>,
        machine: string,
        fetch: (request: Request) => Promise<Response>,
    ) {
        // resolve the current machine's endpoint for every request
        let current = machine;
        const endpoint = async () => {
            const found = await this.endpoint(current);
            if (found === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `machine ${current} publishes no endpoint`,
                });
            }

            return `${found.url.replace(/\/+$/u, "")}${ServiceMount.path(service.package.id)}`;
        };

        return createClient(service, {
            url: endpoint,
            fetch: async (request, options) => {
                // send the request and keep its body for the moved scope's machine
                const sent = new Request(request, options);
                const body = sent.body === null ? null : await sent.clone().arrayBuffer();
                const from = await endpoint();
                const response = await fetch(sent);
                const moved = await Moved.read(response);
                if (moved === undefined) {
                    return response;
                }

                // resend once to the machine the scope moved to
                current = moved.machine;
                const target = `${await endpoint()}${sent.url.slice(from.length)}`;

                return fetch(
                    new Request(target, { method: sent.method, headers: sent.headers, body }),
                );
            },
        });
    }
}
