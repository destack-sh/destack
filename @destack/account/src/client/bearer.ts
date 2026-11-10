import type { Fetch } from "@destack/object";
import { PackageId } from "@destack/package";
import { copyRequest, type Environment } from "@destack/environment";
import { TokenCache } from "@destack/access/token";
import { TOKEN_PATH } from "../authentication/path.ts";
import { TokenResponse } from "../authentication/token.ts";
import type { TokenBinding } from "./issuer.ts";
import { JWT_BEARER } from "../oauth/bearer.ts";

/** A source of a caller's access tokens, by its assertions at a token endpoint (RFC 7523). */
export class TokenSource {
    /** Sign an assertion of the caller for a token endpoint. */
    readonly #assert: (endpoint: string) => Promise<string>;
    /** The tokens granted so far, by audience and partition. */
    readonly #tokens = new TokenCache();
    /** The environment tokens are refreshed at. */
    readonly #environment: Environment;

    /** Get tokens by the assertions a caller signs, in an environment. */
    constructor(assert: (endpoint: string) => Promise<string>, environment: Environment) {
        this.#assert = assert;
        this.#environment = environment;
    }

    /** Wrap a fetch to call a package with the caller's tokens, in a partition when given. */
    fetch(
        send: Fetch,
        audience: PackageId,
        issuer: string,
        binding: Pick<TokenBinding, "partition"> = {},
    ): Fetch {
        return async (request) => {
            // replace the request's credential with the token for the audience
            const accessToken = await this.token(audience, issuer, send, binding);
            const headers = new Headers(request.headers);
            headers.set("authorization", `Bearer ${accessToken}`);

            return send(copyRequest(request, { headers }));
        };
    }

    /** Read the caller's access token for a package from an issuer, in a partition when given. */
    async token(
        audience: PackageId,
        issuer: string,
        fetch: Fetch,
        binding: Pick<TokenBinding, "partition"> = {},
    ): Promise<string> {
        // reuse a pending or fresh token, else ask the token endpoint with a new assertion
        const key = JSON.stringify([audience, binding.partition ?? null]);

        return this.#tokens.token(key, this.#environment.clock.now(), async () =>
            this.#grant(audience, issuer, fetch, binding),
        );
    }

    /** Forget every token granted so far. */
    clear(): void {
        this.#tokens.clear();
    }

    /** Exchange an assertion for an access token at the issuer's token endpoint. */
    async #grant(
        audience: PackageId,
        issuer: string,
        fetch: Fetch,
        binding: Pick<TokenBinding, "partition">,
    ): Promise<string> {
        // post the assertion for the package's resource
        const endpoint = `${issuer}${TOKEN_PATH}`;
        const body = new URLSearchParams({
            grant_type: JWT_BEARER,
            assertion: await this.#assert(endpoint),
            resource: PackageId.resource(audience),
            ...(binding.partition === undefined ? {} : { partition: binding.partition }),
        });
        const response = await fetch(new Request(endpoint, { method: "POST", body }));

        // refuse as the endpoint refused
        if (!response.ok) {
            throw await TokenResponse.refusal(response);
        }

        return TokenResponse.parse(await response.json()).access_token;
    }
}
