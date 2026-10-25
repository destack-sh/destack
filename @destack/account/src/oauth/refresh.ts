import type { DatabaseConnection } from "@destack/db";
import type { Environment } from "@destack/environment";
import type { ObjectStore } from "@destack/object/server";
import { schema } from "@destack/schema";
import { OAuthError } from "../authentication/token.ts";
import { AccountAuthentication } from "../better-auth/authentication.ts";
import { RefreshToken } from "../object/refresh-token.ts";
import { ClientAuthentication } from "./party.ts";
import { PackageResource } from "./resource.ts";
import { spaceSeparated, type TokenGrant } from "./token.ts";

/** A refresh request: the token, and narrower scopes or the package (RFC 6749 6). */
const RefreshRequest = schema.looseObject({
    /** The refresh token. */
    refresh_token: schema.string().min(1),
    /** The scopes asked for, within the granted ones. */
    scope: schema.string().exactOptional(),
    /** The package asked for, the granted one. */
    resource: schema.string().min(1).exactOptional(),
});

/** What the refresh grant reads tokens from and stores their successors in. */
export interface RefreshOptions {
    /** The account database with the tokens and consents. */
    readonly database: DatabaseConnection;
    /** The account objects, which rotate and revoke tokens. */
    readonly objects: Pick<ObjectStore, "mutate" | "as">;
    /** The clock tokens are checked at, the WebCrypto digesting them and the randomness. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
}

/** Renew a client's tokens with its refresh token, rotating it (RFC 6749 6). */
export function refreshGrant(options: RefreshOptions): TokenGrant {
    return async ({ body, headers }) => {
        // authenticate the client and read the request
        const { database, objects, environment } = options;
        const { party, asked } = await ClientAuthentication.grant(
            { body, headers },
            "refresh_token",
            RefreshRequest,
            { database, crypto: environment.crypto },
        );

        // find the client's unexpired token
        const found = await RefreshToken.find(asked.refresh_token, database, environment);
        if (found === undefined || found.consent.clientId !== party.clientId) {
            throw OAuthError.of("INVALID_GRANT", "invalid refresh token");
        }
        const { row, consent } = found;

        // refuse scopes beyond the granted ones
        const scopes = asked.scope === undefined ? row.scopes : spaceSeparated(asked.scope);
        if (scopes.some((scope) => !row.scopes.includes(scope))) {
            throw OAuthError.of("INVALID_SCOPE", "the scopes exceed the granted ones");
        }
        // refuse another package than the granted one
        else if (asked.resource !== undefined && asked.resource !== row.resource) {
            throw OAuthError.of("INVALID_TARGET", `${asked.resource} was not granted`);
        }

        // refuse a token spent before, revoking its family
        if (row.status === "rotated") {
            await RefreshToken.revoke(objects, database, row, environment);
            throw OAuthError.of("INVALID_GRANT", "the refresh token was used before");
        }

        // describe the person through the client, until the token lapses, as offline access outlives the session
        const resource = row.resource ?? undefined;
        const authentication = AccountAuthentication.client(
            { userId: consent.scope, clientId: party.clientId },
            row.expiresAt,
            PackageResource.audience(resource),
            environment.clock.now(),
        );

        return {
            authentication,
            clientId: party.clientId,
            scopes,
            ...(resource === undefined ? {} : { resource }),
            consent: {
                row: consent,
                family: row.family,
                rotating: row,
            },
        };
    };
}
