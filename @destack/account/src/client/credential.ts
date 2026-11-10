import type { Fetch } from "@destack/object";
import { PackageId } from "@destack/package";
import { copyRequest, type Environment } from "@destack/environment";
import { TokenCache } from "@destack/access/token";
import { canonicalize } from "@destack/schema";
import { TOKEN_PATH } from "../authentication/path.ts";
import { TokenResponse } from "../authentication/token.ts";
import type { TokenBinding } from "./issuer.ts";
import { JWT_BEARER } from "../oauth/bearer.ts";

/** What a session calls packages with: a principal's tokens. */
export interface Credential {
    /** Add the principal's token for a package to a request, bound as asked. */
    readonly authorize: (
        request: Request,
        audience: PackageId,
        binding: TokenBinding,
    ) => Promise<Request>;
}

/** Copy a request with a bearer token (RFC 6750). */
export function bearer(request: Request, token: string): Request {
    const headers = new Headers(request.headers);
    headers.set("authorization", `Bearer ${token}`);

    return copyRequest(request, { headers });
}

/** What signs a principal's assertions for a token endpoint (RFC 7523): its key, or a proof. */
export interface Signer {
    /** Sign an assertion of the principal for a token endpoint now. */
    readonly assert: (endpoint: string) => Promise<string>;
}

/** A principal's tokens as an issuer grants them for its assertions, reused until they lapse. */
export class IssuerCredential implements Credential {
    /** The key signing the principal's assertions. */
    readonly signer: Signer;
    /** The issuer granting the tokens. */
    readonly issuer: string;
    /** The tokens granted so far, by audience and partition. */
    readonly #tokens = new TokenCache();
    /** The fetch calling the issuer. */
    readonly #fetch: Fetch;
    /** The environment tokens are reused by. */
    readonly #environment: Environment;

    /** Get a principal's tokens from an issuer by the assertions its key signs. */
    constructor(signer: Signer, issuer: string, fetch: Fetch, environment: Environment) {
        // keep the signer, the issuer, the fetch and the environment
        this.signer = signer;
        this.issuer = issuer;
        this.#fetch = fetch;
        this.#environment = environment;
    }

    /** Add the principal's granted token for a package to a request, in a partition when bound. */
    async authorize(
        request: Request,
        audience: PackageId,
        binding: TokenBinding,
    ): Promise<Request> {
        return bearer(request, await this.token(audience, binding));
    }

    /** Read the principal's granted token for a package, in a partition when bound. */
    token(audience: PackageId, binding: Pick<TokenBinding, "partition">): Promise<string> {
        const key = canonicalize([audience, binding.partition ?? null]);

        return this.#tokens.token(key, this.#environment.clock.now(), () =>
            this.#grant(audience, binding),
        );
    }

    /** Forget every token granted so far, as a replaced key's tokens no longer hold. */
    clear(): void {
        this.#tokens.clear();
    }

    /** Exchange an assertion for an access token at the issuer's token endpoint. */
    async #grant(audience: PackageId, binding: Pick<TokenBinding, "partition">): Promise<string> {
        // post the assertion for the package's resource
        const endpoint = `${this.issuer}${TOKEN_PATH}`;
        const body = new URLSearchParams({
            grant_type: JWT_BEARER,
            assertion: await this.signer.assert(endpoint),
            resource: PackageId.resource(audience),
            ...(binding.partition === undefined ? {} : { partition: binding.partition }),
        });
        const response = await this.#fetch(new Request(endpoint, { method: "POST", body }));

        // refuse as the endpoint refused
        if (!response.ok) {
            throw await TokenResponse.refusal(response);
        }

        return TokenResponse.parse(await response.json()).access_token;
    }
}
