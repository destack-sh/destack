import { JWKS_PATH } from "@destack/account/authentication";
import { AccountAuthentication, type Authenticator } from "@destack/account/better-auth";
import type { WorkloadIdentity } from "@destack/account/client";
import { HostKey } from "@destack/account/object";
import type { DatabaseConnection } from "@destack/db";
import { type Directory, DirectoryStore } from "@destack/directory";
import { SpaceToken } from "@destack/host/identity";
import type { PackageId } from "@destack/package";
import { type Authentication, Bearer, TokenVerifier } from "@destack/service/authentication";

/** The verification of a platform process's callers by space token, universe token or account credential. */
export class PlatformAuthentication {
    /** The universe's issuer, the account service's origin. */
    readonly #issuer: string;
    /** The fetch reaching the issuer's key set. */
    readonly #fetch: (request: Request) => Promise<Response>;
    /** The verifiers of the universe's tokens, by audience. */
    readonly #verifiers = new Map<string, TokenVerifier>();

    /** Verify callers against an issuer's tokens, reaching its key set through a fetch. */
    constructor(issuer: string, fetch: (request: Request) => Promise<Response>) {
        this.#issuer = issuer;
        this.#fetch = fetch;
    }

    /** Verify a caller of the account service over its own database. */
    async account(
        request: Request,
        audience: PackageId,
        signIn: Authenticator,
        database: DatabaseConnection,
    ): Promise<Authentication | null> {
        // verify a space token or a universe token against the account service's records
        if (PlatformAuthentication.#isSigned(request)) {
            return this.#token(request, audience, new DirectoryStore(database), database);
        }

        return AccountAuthentication.authenticate(request, signIn, audience);
    }

    /** Verify a caller of a workload that runs apart from the account service. */
    async workload(
        request: Request,
        audience: PackageId,
        identity: WorkloadIdentity,
        database: DatabaseConnection,
    ): Promise<Authentication | null> {
        // call anonymously without a bearer credential
        if (!request.headers.has("authorization")) {
            return null;
        }
        // verify a space token or a universe token as the workload
        else if (PlatformAuthentication.#isSigned(request)) {
            return this.#token(request, audience, identity.directory(), database);
        }
        // introspect an account credential at the account service
        else {
            return identity.introspect(Bearer.require(request.headers), request.signal);
        }
    }

    /** Read the verifier of the universe's tokens for an audience, caching its key set between fetches. */
    verifier(audience: PackageId): TokenVerifier {
        const verifier =
            this.#verifiers.get(audience) ??
            new TokenVerifier({
                authority: { kind: "universe" },
                issuer: this.#issuer,
                audience,
                keys: new URL(JWKS_PATH, this.#issuer),
                fetch: (input, options) =>
                    this.#fetch(
                        input instanceof Request ? input : new Request(input.toString(), options),
                    ),
            });
        this.#verifiers.set(audience, verifier);

        return verifier;
    }

    /** Verify a space token through a directory, or a universe token whose host key stands in a database. */
    async #token(
        request: Request,
        audience: PackageId,
        directory: Directory,
        database: DatabaseConnection,
    ): Promise<Authentication> {
        // verify a space token by the space's identity
        if (SpaceToken.accepts(request)) {
            return SpaceToken.verify(request, { directory, audience });
        }

        // verify a universe token, whose host key stands
        const caller = await this.verifier(audience).authenticate(request);
        await HostKey.requireAuthenticating(database, caller, Date.now());

        return caller;
    }

    /** Report whether a request presents a signed token, a space's or the universe's: a bearer JWT. */
    static #isSigned(request: Request): boolean {
        const token = Bearer.token(request.headers.get("authorization"));

        return token !== undefined && token.split(".").length === 3;
    }
}
