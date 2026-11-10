import { principal, type StepUp } from "@destack/access";
import type { Environment } from "@destack/environment";
import type { PackageId } from "@destack/package";
import { Scope, type Subject } from "@destack/sync";
import { canonicalize, type Identifier } from "@destack/schema";
import type { TokenCache } from "@destack/access/token";
import accountPackage from "../package.ts";
import { ACCESS_TOKEN_TYPE } from "../oauth/exchange.ts";
import { bearer, type Credential } from "./credential.ts";
import type { Issuer, TokenBinding } from "./issuer.ts";

/** Where a user's account tokens come from, and the cache of the tokens exchanged for them. */
export interface UserTokens {
    /** The cache of the user's access tokens, by package and binding. */
    readonly cache: TokenCache;
    /** Issue the user's account access token anew. */
    readonly issue: () => Promise<string>;
}

/** A signed-in user's tokens: their account token, exchanged at their issuer for each package. */
export class UserIdentity implements Credential {
    /** The user's identifier. */
    readonly userId: Identifier<"user">;
    /** The issuer that signed the user in. */
    readonly issuer: Issuer;
    /** Where the user's account tokens come from. */
    readonly #tokens: UserTokens;
    /** The environment tokens are reused by. */
    readonly #environment: Environment;

    /** Act as a signed-in user at their issuer through their account tokens. */
    constructor(
        userId: Identifier<"user">,
        issuer: Issuer,
        tokens: UserTokens,
        environment: Environment,
    ) {
        // keep the user, their issuer, their tokens and the environment
        this.userId = userId;
        this.issuer = issuer;
        this.#tokens = tokens;
        this.#environment = environment;
    }

    /** The user. */
    get subject(): Subject {
        return principal.user.reference(Scope.universe.id, this.userId);
    }

    /** Add the user's token for a package to a request, bound as asked. */
    async authorize(
        request: Request,
        audience: PackageId,
        binding: TokenBinding,
    ): Promise<Request> {
        return bearer(request, await this.token(audience, binding));
    }

    /** Read the user's token for a package under a binding, answering a step-up. */
    async token(audience: PackageId, binding: TokenBinding = {}, stepUp?: StepUp): Promise<string> {
        // take the account token itself toward the account package outside spaces
        const isUnbound = Object.keys(binding).length === 0;
        if (audience === accountPackage.id && isUnbound) {
            return this.accountToken(stepUp);
        }

        // exchange it for the package's token, cached per package and binding
        return this.#tokens.cache.token(
            canonicalize([audience, binding]),
            this.#environment.clock.now(),
            async () => {
                const token = await this.accountToken(stepUp);
                const exchanged = await this.issuer.exchange(
                    { token, type: ACCESS_TOKEN_TYPE },
                    audience,
                    binding,
                );

                return exchanged.accessToken;
            },
            stepUp,
        );
    }

    /** Read the user's account token, renewed for a step-up. */
    accountToken(stepUp?: StepUp): Promise<string> {
        return this.#tokens.cache.token(
            accountPackage.id,
            this.#environment.clock.now(),
            () => this.#tokens.issue(),
            stepUp,
        );
    }
}
