import { type Authentication, Caller, principal } from "@destack/access";
import type {
    OAuthExtensionGrantHandlerInput,
    OAuthTokenResponse,
} from "@better-auth/oauth-provider";
import type { JWTPayload } from "jose";
import { PackageId } from "@destack/package";
import {
    ACCESS_TOKEN_TYPE,
    TOKEN_LIFETIME_MILLISECONDS,
    TokenIdentity,
    TokenIssuerAuthority,
} from "@destack/runtime/authentication";
import { APIError } from "better-auth/api";

/** The milliseconds in a second, as OAuth counts expiries in seconds. */
const SECOND_MILLISECONDS = 1000;

/** The provider's default lifetime of a token without a person, in seconds. */
const MACHINE_TOKEN_SECONDS = 3600;

/** The access tokens the token endpoint issues: Better Auth's, carrying the Destack claims. */
export const Issuance = {
    /** Build the Destack claims of an authentication within the universe's authority. */
    claims(authentication: Authentication) {
        TokenIssuerAuthority.require({ kind: "universe" }, authentication);

        return TokenIdentity.claims(authentication);
    },

    /** Issue a caller's token for a package, refusing one outliving its session or grant. */
    async issue(
        input: Pick<OAuthExtensionGrantHandlerInput, "ctx" | "provider">,
        authentication: Authentication,
        terms: {
            /** The time of the issuance. */
            readonly now: number;
            /** The lifetimes of the packages' tokens, a minute when absent. */
            readonly lifetimes: ReadonlyMap<PackageId, number>;
            /** When the session or grant the token acts under lapses, none for neither. */
            readonly lapse?: number;
            /** The OAuth scopes granted beside the package: none, or a person's sign-in's. */
            readonly scopes: readonly string[];
            /** The session the tokens belong to, which describes the person at each refresh. */
            readonly session?: string;
        },
    ): Promise<OAuthTokenResponse> {
        // refuse a token the provider would let outlive the session or grant it acts under
        const { caller, credential } = authentication;
        const { audience } = credential;
        const lifetime = terms.lifetimes.get(audience) ?? TOKEN_LIFETIME_MILLISECONDS;
        if (terms.lapse !== undefined && terms.lapse < terms.now + lifetime) {
            throw Issuance.refusal(
                "invalid_grant",
                "the session or grant lapses before the token would",
            );
        }

        // issue to the caller's client, else to the keyed principal sending the calls (R1)
        const { subject } = caller;
        const sender = Caller.sender(caller);
        const isPerson = principal.user.is(subject);
        const client = credential.client ?? (principal.user.is(sender) ? undefined : sender.id);
        if (client === undefined) {
            throw Issuance.refusal("invalid_grant", "a user's token needs a client");
        }

        // name the person the token represents, refusing one Better Auth does not know
        const person = isPerson
            ? await input.ctx.context.internalAdapter.findUserById(subject.id)
            : undefined;
        if (person === null) {
            throw Issuance.refusal("invalid_grant", `no user ${subject.id}`);
        }

        return input.provider.issueTokens({
            client: { clientId: client },
            ...(person === undefined ? {} : { user: person }),
            scopes: [...terms.scopes],
            ...(terms.session === undefined
                ? {}
                : {
                      sessionId: terms.session,
                      ...(credential.assurance === undefined
                          ? {}
                          : { authTime: new Date(credential.assurance.authenticatedAt) }),
                  }),
            resources: [PackageId.resource(credential.audience)],
            accessTokenClaims: Issuance.claims(authentication),
        });
    },

    /** Sign a key holder's token for a package at the store's time, with the provider's claims. */
    async sign(
        input: Pick<OAuthExtensionGrantHandlerInput, "ctx">,
        authentication: Authentication,
        terms: {
            /** The time of the issuance. */
            readonly now: number;
            /** The lifetimes of the packages' tokens, a minute when absent. */
            readonly lifetimes: ReadonlyMap<PackageId, number>;
            /** The universe's issuer, which its tokens carry. */
            readonly issuer: string;
            /** Sign claims with the universe's active key, as the provider signs its tokens. */
            readonly sign: (
                claims: JWTPayload,
                header: { readonly typ: string },
            ) => Promise<string>;
        },
    ): Promise<OAuthTokenResponse> {
        // time the token by the package's lifetime, capped at the provider's machine default
        const { caller, credential } = authentication;
        const { audience } = credential;
        const lifetime = terms.lifetimes.get(audience) ?? TOKEN_LIFETIME_MILLISECONDS;
        const issuedAt = Math.floor(terms.now / SECOND_MILLISECONDS);
        const expiresAt =
            issuedAt + Math.min(lifetime / SECOND_MILLISECONDS, MACHINE_TOKEN_SECONDS);

        // issue to the caller's client, else to the key holder sending the calls
        const client = credential.client ?? Caller.sender(caller).id;
        const claims = {
            ...Issuance.claims(authentication),
            sub: client,
            aud: PackageId.resource(audience),
            client_id: client,
            azp: client,
            scope: "",
            iss: terms.issuer,
            iat: issuedAt,
            exp: expiresAt,
            jti: crypto.randomUUID().replaceAll("-", ""),
        };
        const accessToken = await terms.sign(claims, { typ: ACCESS_TOKEN_TYPE });

        return input.ctx.json({
            access_token: accessToken,
            expires_in: expiresAt - issuedAt,
            expires_at: expiresAt,
            token_type: "Bearer",
            refresh_token: undefined,
            scope: "",
            id_token: undefined,
        });
    },

    /** Build a token endpoint refusal (RFC 6749 5.2). */
    refusal(error: "invalid_request" | "invalid_grant" | "invalid_target", description: string) {
        return new APIError("BAD_REQUEST", { error, error_description: description });
    },
};
