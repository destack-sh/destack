import type { TokenLifetime } from "./resource.ts";
import {
    type ClientDiscovery,
    DEVICE_CODE_GRANT_TYPE,
    type OAuthClaimExtensionInput,
    type OAuthProviderExtension,
} from "@better-auth/oauth-provider";
import { type Authentication, type Authorizer, principal, Caller } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import type { JWTPayload } from "jose";
import { Scope } from "@destack/sync";
import { AccountAuthentication } from "../better-auth/authentication.ts";
import { ActiveSession } from "../better-auth/active-session.ts";
import { Client, client, DEVICE_CLIENT_NAME } from "../object/client.ts";
import accountPackage from "../package.ts";
import { bearerGrant, JWT_BEARER } from "./bearer.ts";
import { type CodeCheck, codeGrant, EMAIL_CODE } from "./code.ts";
import { exchangeGrant, TOKEN_EXCHANGE } from "./exchange.ts";
import { Issuance } from "./issuance.ts";
import type { Clock } from "@destack/environment";

/** A client a discovery source resolves, as the provider reads it. */
type DiscoveredClient = NonNullable<Awaited<ReturnType<ClientDiscovery["resolve"]>>>;

/** The loopback redirect a device's client returns to, on any port (RFC 8252 7.3). */
export const LOOPBACK_REDIRECT = "http://127.0.0.1/callback";

/** What the token endpoint's Destack grants verify and read with. */
export interface OAuthExtensionOptions {
    /** The account database. */
    readonly database: DatabaseConnection;
    /** The account objects, which register devices' clients. */
    readonly objects: Pick<ObjectStore, "mutate" | "database" | "serving">;
    /** The check of mailed sign-in codes, spending each. */
    readonly codes: CodeCheck;
    /** The policies expanding memberships. */
    readonly authorizer: Authorizer;
    /** The universe's issuer, the oauth adapter's URL. */
    readonly issuer: string;
    /** The token endpoint's URL, every assertion's audience. */
    readonly endpoint: string;
    /** The clock tokens are issued at. */
    readonly clock: Clock;
    /** How long the packages' tokens live. */
    readonly lifetime: TokenLifetime;
    /** Sign claims with the universe's active key, as the provider signs its tokens. */
    readonly sign: (claims: JWTPayload, header: { readonly typ: string }) => Promise<string>;
}

/** Extend the provider with Destack's grants and the Destack claims of its grants' tokens. */
export function oauthExtension(options: OAuthExtensionOptions): OAuthProviderExtension {
    return {
        grants: {
            [JWT_BEARER]: bearerGrant(options),
            [TOKEN_EXCHANGE]: exchangeGrant(options),
            [EMAIL_CODE]: codeGrant({ ...options, check: options.codes }),
        },
        claims: { accessToken: (input) => personClaims(options, input) },
        clientDiscovery: {
            id: "destack-client",
            matches: (clientId) => client.identifies(clientId),
            resolve: async (_context, clientId) =>
                (await Client.isAdmissible(options.database, clientId))
                    ? deviceClient(clientId)
                    : null,
        },
    };
}

/** Describe a device's client as a public native OAuth client (RFC 8252). */
function deviceClient(clientId: string): DiscoveredClient {
    return {
        clientId,
        name: DEVICE_CLIENT_NAME,
        applicationType: "native",
        tokenEndpointAuthMethod: "none",
        redirectUris: [LOOPBACK_REDIRECT],
        grantTypes: ["authorization_code", "refresh_token", DEVICE_CODE_GRANT_TYPE],
        responseTypes: ["code"],
        requirePKCE: true,
        scopes: ["openid", "profile", "email", "offline_access"],
    };
}

/** Build the Destack claims of a person's token a provider grant issues, such as a device code. */
async function personClaims(
    options: OAuthExtensionOptions,
    input: OAuthClaimExtensionInput,
): Promise<Record<string, unknown>> {
    // leave a token without a person to the grant issuing it
    const { user, sessionId } = input;
    if (user === undefined || user === null) {
        return {};
    }

    // describe the person through the session that approved the client, else through the client
    const now = options.clock.now();
    const authentication: Authentication =
        sessionId === undefined || sessionId === null
            ? described(user.id, input.client.clientId, now)
            : AccountAuthentication.activeSession(
                  await ActiveSession.read(sessionId, options.database, now),
                  accountPackage.id,
                  now,
              );

    return Issuance.claims(authentication);
}

/** Describe a person a client acts for without a session, by the client's credential. */
function described(userId: string, clientId: string, now: number): Authentication {
    const subject = principal.user.reference(Scope.universe.id, userId);

    return {
        caller: Caller.of(subject),
        credential: {
            kind: "client",
            id: clientId,
            audience: accountPackage.id,
            verifiedAt: now,
            expiresAt: now,
        },
    };
}
