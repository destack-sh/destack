import {
    InstallationMount,
    SpaceMount,
    ServiceMount,
    type Service,
    type ServiceRouter,
} from "@destack/service";
import type { Caller } from "@destack/access";
import { Bearer, TokenVerifier } from "@destack/service/authentication";
import type { PackageId } from "@destack/package";
import { type Clock, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { createClient } from "@destack/service/client";
import { ServiceError } from "@destack/service/error";
import { PublicKey } from "@destack/identity";
import { createLocalJWKSet, errors, type JWTVerifyGetKey } from "jose";
import type { Directory } from "./directory.ts";

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

/** A DID resolver over the universe's directory: the keys an identity signs with and the URLs an address answers at, kept and verified against. */
export class Resolver {
    /** The directory resolved through. */
    readonly directory: Directory;
    /** The clock key sets are kept by. */
    readonly #clock: Clock;
    /** The key sets verifiers read, by subject. */
    readonly #keySets = new Map<string, Promise<KeptKeySet>>();

    /** Resolve through a directory, keeping key sets by a clock. */
    constructor(directory: Directory, clock: Clock) {
        this.directory = directory;
        this.#clock = clock;
    }

    /** Read the keys verifying what an identity signs, kept for ten minutes and read again for a key it lacks, as a remote JSON Web Key Set is. */
    keys(subject: string): JWTVerifyGetKey {
        return async (header, token) => {
            // read the key set once its keeping lapsed
            const now = this.#clock.now();
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
    authenticate(request: Request, options: Parameters<Resolver["verify"]>[1]): Promise<Caller> {
        return this.verify(Bearer.require(request.headers), options);
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
    ): Promise<Caller> {
        // read the issuing space, or the universe
        const now = this.#clock.now();
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

        return verifier.verify(token, now, options.scope);
    }

    /** Read an identity's key set, kept unless it is older than an age. */
    #keySet(subject: string, now: number, age: number): Promise<KeptKeySet> {
        // keep a key set read within the age, and forget a failed read unless a newer one replaced it
        const kept = this.#keySets.get(subject);
        const reading =
            kept === undefined
                ? this.#readKeySet(subject, now)
                : kept.then((keySet) =>
                      now - keySet.readAt < age ? keySet : this.#readKeySet(subject, now),
                  );
        this.#keySets.delete(subject);
        this.#keySets.set(subject, reading);
        reading.catch(() => {
            if (this.#keySets.get(subject) === reading) {
                this.#keySets.delete(subject);
            }
        });
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
        const identity = await this.directory.identity(subject);
        if (identity === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: `${subject} has no identity` });
        }

        return {
            keys: createLocalJWKSet(await PublicKey.keySet(identity.signingKeys)),
            readAt: now,
        };
    }

    /** Create a client of a service at an address, resolving where it is served once and again after a misdirected answer, which it follows once. */
    client<Router extends ServiceRouter>(
        service: Service<Router>,
        address: string,
        fetch: (request: Request) => Promise<Response>,
    ) {
        // resolve the address once, and again after a misdirected answer
        const mount = ServiceMount.path(service.package.id);
        let resolved: string | undefined;

        return createClient(service, {
            url: async () => (resolved ??= `${await this.url(address)}${mount}`),
            fetch: async (request, options) => {
                // keep the body for a second attempt
                const sent = new Request(request, options);
                const body = sent.body === null ? null : await sent.clone().arrayBuffer();
                const below = sent.url.slice(present(resolved, "the resolved URL").length);

                // send, and resend once where the address resolves after a misdirected answer
                return ServiceError.follow(async (misdirected) =>
                    misdirected === undefined
                        ? fetch(sent)
                        : fetch(
                              new Request(
                                  `${(resolved = `${await this.url(address)}${mount}`)}${below}`,
                                  { method: sent.method, headers: sent.headers, body },
                              ),
                          ),
                );
            },
        });
    }

    /** Find the URL below which an address mounts its services on the machine serving it now: a machine's own (`<machine>`), a space's (`<space>`), or an installation's by identifier or alias (`<installation>--<space>`). */
    async url(address: string): Promise<string> {
        // take a machine's own services at its endpoint
        const [inner, space, ...rest] = address.split("--");
        const machine = schema.identifier("machine").safeParse(inner);
        if (space === undefined && machine.success) {
            const found = await this.directory.endpoint(machine.data);
            if (found === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `machine ${machine.data} publishes no endpoint`,
                });
            }

            return found.url.replace(/\/+$/u, "");
        }
        // take a space's own services below the space
        else if (space === undefined && inner !== undefined) {
            return SpaceMount.url(await this.#served(inner), inner);
        }
        // take an installation's services below the installation
        else if (inner !== undefined && space !== undefined && rest.length === 0) {
            return InstallationMount.url(await this.#served(space), space, inner);
        }
        // refuse anything else
        else {
            throw new ServiceError("NOT_FOUND", { message: `no address ${address}` });
        }
    }

    /** Find the endpoint URL of the machine serving a space, refusing a space no machine serves. */
    async #served(space: string): Promise<string> {
        // find the space's machine and its endpoint
        const placed = await this.directory.locate(space);
        const found =
            placed === undefined ? undefined : await this.directory.endpoint(placed.machine);
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no machine serves ${space}` });
        }

        return found.url;
    }
}
