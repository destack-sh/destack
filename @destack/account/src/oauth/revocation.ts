import { type DatabaseConnection, Snapshot } from "@destack/db";
import { schema } from "@destack/schema";
import type { Environment } from "@destack/environment";
import type { ObjectStore } from "@destack/object/server";
import { OAuthError, SECOND_MILLISECONDS } from "../authentication/token.ts";
import { RefreshToken } from "../object/refresh-token.ts";
import { client } from "../object/client.ts";
import { User, user } from "../object/user.ts";
import { ActiveSession } from "../better-auth/active-session.ts";
import { AccountError } from "../error/error.ts";
import { AccessToken, type AccessTokenOptions, TokenEndpoint } from "./token.ts";
import { ClientAuthentication, type OAuthClient } from "./party.ts";

/** The dot-separated segments of a signed JWT: header, payload and signature (RFC 7519 3). */
const JWT_SEGMENTS = 3;

/** What revocation and introspection read tokens with. */
export interface TokenReadOptions extends AccessTokenOptions {
    /** The account database with the refresh tokens and consents. */
    readonly database: DatabaseConnection;
    /** The account objects, which revoke refresh tokens. */
    readonly objects: Pick<ObjectStore, "mutate" | "as">;
    /** The clock, the WebCrypto and the randomness of revocations' identifiers. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
}

/** The revocation endpoint: end a client's refresh token and its family (RFC 7009). */
export class RevocationEndpoint {
    /** What the endpoint reads tokens with. */
    readonly #options: TokenReadOptions;

    /** Answer revocation requests. */
    constructor(options: TokenReadOptions) {
        this.#options = options;
    }

    /** Revoke a client's refresh token, answering success for an unknown one (RFC 7009 2.2). */
    async answer(request: Request): Promise<Response> {
        // authenticate the client and read the token
        const { database, objects, environment } = this.#options;
        const body = await TokenEndpoint.form(request);
        const party = await ClientAuthentication.verify(
            body,
            request.headers,
            database,
            environment.crypto,
        );
        const token = body["token"];
        if (token === undefined) {
            throw OAuthError.of("INVALID_REQUEST", "the request names no token");
        }

        // refuse an access token, which lives its short lifetime out
        if (token.split(".").length === JWT_SEGMENTS) {
            throw OAuthError.of("UNSUPPORTED_TOKEN_TYPE", "access tokens lapse at their expiry");
        }

        // revoke the family of the client's refresh token
        const row = await readRefreshToken(token, party, this.#options);
        if (row !== undefined) {
            await RefreshToken.revoke(objects, database, row, environment);
        }

        return new Response(null, { status: 200 });
    }
}

/** The introspection endpoint: describe a token to a client (RFC 7662). */
export class IntrospectionEndpoint {
    /** What the endpoint reads tokens with. */
    readonly #options: TokenReadOptions;

    /** Answer introspection requests. */
    constructor(options: TokenReadOptions) {
        this.#options = options;
    }

    /** Describe a token: a refresh token of the client, or an access token the issuer signed. */
    async answer(request: Request): Promise<Response> {
        // authenticate the client and read the token
        const { database, environment, issuer } = this.#options;
        const body = await TokenEndpoint.form(request);
        const party = await ClientAuthentication.verify(
            body,
            request.headers,
            database,
            environment.crypto,
        );
        const token = body["token"];
        if (token === undefined) {
            throw OAuthError.of("INVALID_REQUEST", "the request names no token");
        }

        // describe a live access token by its claims
        if (token.split(".").length === JWT_SEGMENTS) {
            const claims = await AccessToken.verify(token, undefined, this.#options).catch(
                (error: unknown) => {
                    // pass on a failure other than an invalid token
                    if (!OAuthError.is(error, "INVALID_TOKEN")) {
                        throw error;
                    }

                    return undefined;
                },
            );

            // describe the asking client's own token alone, while what it rests on stands (RFC 7662 2.2, 4)
            const isOwn = claims !== undefined && claims["client_id"] === party.clientId;
            const isActive = isOwn && (await this.#isStanding(claims));

            return Response.json(
                isActive
                    ? { ...claims, active: true, token_type: "access_token" }
                    : { active: false },
            );
        }

        // describe the client's live refresh token
        const row = await readRefreshToken(token, party, this.#options);

        return Response.json(
            row === undefined || row.status !== "live"
                ? { active: false }
                : {
                      active: true,
                      token_type: "refresh_token",
                      client_id: party.clientId,
                      sub: row.scope,
                      scope: row.scopes.join(" "),
                      iss: issuer,
                      exp: Math.floor(row.expiresAt / SECOND_MILLISECONDS),
                  },
        );
    }

    /** Report whether a token's session, else its user and device client, still stand. */
    async #isStanding(claims: Readonly<Record<string, unknown>>): Promise<boolean> {
        // read the session a session-bound token rests on, refusing one that ended
        const { database, environment } = this.#options;
        const { sub, sid, client_id: clientId } = StandingClaims.parse(claims);
        if (sid !== undefined) {
            return ActiveSession.read(sid, database, environment.clock.now()).then(
                () => true,
                (error: unknown) => {
                    if (!AccountError.is(error, "SESSION_ENDED")) {
                        throw error;
                    }

                    return false;
                },
            );
        }

        // read the user a client-bound token acts for, and its device client, unrevoked
        const mayUserSignIn = !user.identifies(sub) || (await User.maySignIn(database, sub));
        const isClientStanding =
            !client.identifies(clientId) ||
            (await Snapshot.live(database)
                .read(client.table)
                .findFirst({
                    where: { id: client.identifier(clientId), revokedAt: { isNull: true } },
                })) !== undefined;

        return mayUserSignIn && isClientStanding;
    }
}

/** The claims of an access token introspection reads its standing from. */
const StandingClaims = schema.looseObject({
    /** The person or caller. */
    sub: schema.string(),
    /** The session the token was issued in, absent for one resting on its client. */
    sid: schema.string().exactOptional(),
    /** The client the token was issued to. */
    client_id: schema.string(),
});

/** Read a client's unexpired refresh token, none for another client's or an unknown one. */
async function readRefreshToken(
    token: string,
    party: OAuthClient,
    options: Pick<TokenReadOptions, "database" | "environment">,
): Promise<RefreshToken | undefined> {
    const found = await RefreshToken.find(token, options.database, options.environment);

    return found?.consent.clientId === party.clientId ? found.row : undefined;
}
