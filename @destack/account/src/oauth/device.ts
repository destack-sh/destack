import { type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { type Environment, Random } from "@destack/environment";
import type { ObjectStore } from "@destack/object/server";
import { Digest, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { OAuthError, SECOND_MILLISECONDS } from "../authentication/token.ts";
import { AccountAuthentication } from "../better-auth/authentication.ts";
import { ActiveSession } from "../better-auth/active-session.ts";
import { AccountError } from "../error/error.ts";
import { Client } from "../object/client.ts";
import {
    DeviceAuthorization,
    type DeviceDecision,
    deviceAuthorization,
} from "../object/device-authorization.ts";
import { OAuthConsent, oauthConsent } from "../object/oauth-consent.ts";
import { ClientAuthentication, DEVICE_CODE, OAuthClient } from "./party.ts";
import { PackageResource } from "./resource.ts";
import { spaceSeparated, type TokenGrant, TokenEndpoint } from "./token.ts";
import { Credential } from "../credential/credential.ts";
import type { Decision } from "./authorization.ts";

/** How long a device's codes wait for the person, in milliseconds: half an hour (RFC 8628 3.2). */
const DEVICE_MILLISECONDS = 30 * 60_000;

/** The least wait between a device's polls, in milliseconds: five seconds (RFC 8628 3.2). */
const INTERVAL_MILLISECONDS = 5000;

/** The characters of a user code: consonants alone, which spell no words (RFC 8628 6.1). */
const USER_CODE_ALPHABET = "BCDFGHJKLMNPQRSTVWXZ";

/** The characters of each half of a user code, about 34 bits in all (RFC 8628 6.1). */
const USER_CODE_HALF = 4;

/** A device authorization request: the scopes and package the device asks for (RFC 8628 3.1). */
const DeviceRequest = schema.looseObject({
    /** The asked scopes, space separated. */
    scope: schema.string().exactOptional(),
    /** The package the tokens are for, as its resource. */
    resource: schema.string().min(1).exactOptional(),
});

/** A device's token request: its device code (RFC 8628 3.4). */
const DeviceTokenRequest = schema.looseObject({
    /** The device code. */
    device_code: schema.string().min(1),
});

/** What the device flow reads, stores and pages with. */
export interface DeviceOptions {
    /** The account database. */
    readonly database: DatabaseConnection;
    /** The account objects, which store the codes, consents and devices' clients. */
    readonly objects: Pick<ObjectStore, "mutate" | "as" | "database" | "environment">;
    /** The clock, the randomness of codes and the WebCrypto digesting them. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
    /** The page people enter user codes on. */
    readonly verification: URL;
    /** Read the session a browser's request signs in, none for a signed-out browser. */
    readonly session: (headers: Headers) => Promise<string | undefined>;
}

/** A device's sign-in a person reviews: its stored codes, client and decision. */
export interface DeviceReview {
    /** The stored codes. */
    readonly row: DeviceAuthorization;
    /** The client asking. */
    readonly party: OAuthClient;
    /** The person's decision so far. */
    readonly decision: DeviceDecision;
}

/** The device authorization endpoint: issue codes a person approves (RFC 8628 3.1). */
export class DeviceAuthorizationEndpoint {
    /** What the flow reads, stores and pages with. */
    readonly #options: DeviceOptions;

    /** Issue devices' codes. */
    constructor(options: DeviceOptions) {
        this.#options = options;
    }

    /** Answer a device authorization request with its codes and the page to enter them on. */
    async answer(request: Request): Promise<Response> {
        // authenticate the client and read the request
        const { database, objects, environment, verification } = this.#options;
        const body = await TokenEndpoint.form(request);
        const { party, asked } = await ClientAuthentication.grant(
            { body, headers: request.headers },
            DEVICE_CODE,
            DeviceRequest,
            { database, crypto: environment.crypto },
        );
        const { scope, resource } = asked;
        party.requireScopes(spaceSeparated(scope));
        if (resource !== undefined && PackageResource.package(resource) === undefined) {
            throw OAuthError.of("INVALID_TARGET", `${resource} names no package`);
        }

        // draw the codes and store their digests
        const deviceCode = await Credential.create("device-code", environment);
        const userCode = `${userCodeHalf(environment)}-${userCodeHalf(environment)}`;
        await objects.mutate(objects.as(deviceAuthorization), {
            id: RequestId.create(environment),
            calls: [
                deviceAuthorization.call("create", {
                    ...deviceAuthorization.in(Scope.universe.id),
                    deviceDigest: deviceCode.digest,
                    userDigest: await Digest.of(normalize(userCode), environment.crypto),
                    clientId: party.clientId,
                    scopes: spaceSeparated(scope),
                    resource: resource ?? null,
                    interval: INTERVAL_MILLISECONDS,
                    expiresAt: environment.clock.now() + DEVICE_MILLISECONDS,
                }),
            ],
        });

        // address the verification page, with the user code filled in
        const complete = new URL(verification);
        complete.searchParams.set("user_code", userCode);

        return Response.json({
            device_code: deviceCode.secret,
            user_code: userCode,
            verification_uri: verification.href,
            verification_uri_complete: complete.href,
            expires_in: DEVICE_MILLISECONDS / SECOND_MILLISECONDS,
            interval: INTERVAL_MILLISECONDS / SECOND_MILLISECONDS,
        });
    }

    /** Read the device sign-in a user code asks for, refusing an unknown or lapsed code. */
    async review(userCode: string): Promise<DeviceReview> {
        // read the codes by the user code's digest
        const { database, environment } = this.#options;
        const digest = await Digest.of(normalize(userCode), environment.crypto);
        const [row] = await database
            .select()
            .from(deviceAuthorization.table)
            .where(eq(deviceAuthorization.table.userDigest, digest))
            .limit(1);
        if (row === undefined || row.expiresAt <= environment.clock.now()) {
            throw AccountError.of("DEVICE_CODE_INVALID", "the code is wrong or expired");
        }

        return {
            row,
            party: await OAuthClient.find(row.clientId, database),
            decision: DeviceAuthorization.decision(row),
        };
    }

    /** Approve or deny a pending device sign-in as the signed-in person, after a recent sign-in. */
    async decide(userCode: string, headers: Headers, decision: Decision): Promise<void> {
        // require a recent sign-in with the person's enrolled factor
        const { database, objects, environment } = this.#options;
        const now = environment.clock.now();
        const sessionId = await this.#options.session(headers);
        if (sessionId === undefined) {
            throw AccountError.of("SIGN_IN_REQUIRED", "the person is not signed in");
        }
        const session = await ActiveSession.read(sessionId, database, now);
        await session.requireAuthentication(database, now);

        // refuse a decided sign-in
        const { row, party, decision: reviewed } = await this.review(userCode);
        if (reviewed.status !== "pending") {
            throw AccountError.of(
                "DEVICE_AUTHORIZATION_DECIDED",
                `the device sign-in was ${reviewed.status}`,
            );
        }

        // deny it
        const target = deviceAuthorization.target(row);
        if (decision === "deny") {
            await objects.mutate(objects.as(deviceAuthorization), {
                id: RequestId.create(environment),
                calls: [deviceAuthorization.call("deny", target)],
            });

            return;
        }

        // register a device's client for the person, grant the client the scopes, and approve
        if (party.registration === undefined) {
            await Client.register(objects, party.clientId, session.userId, now);
        }
        await OAuthConsent.grant(
            objects,
            { clientId: party.clientId, userId: session.userId, scopes: row.scopes },
            now,
        );
        await objects.mutate(objects.as(deviceAuthorization), {
            id: RequestId.create(environment),
            calls: [deviceAuthorization.call("approve", { ...target, userId: session.userId })],
        });
    }
}

/** Redeem a device code for the person's tokens once they approved it (RFC 8628 3.4). */
export function deviceGrant(options: Omit<DeviceOptions, "verification" | "session">): TokenGrant {
    return async ({ body, headers }) => {
        // authenticate the client and read its device code
        const { database, objects, environment } = options;
        const { party, asked } = await ClientAuthentication.grant(
            { body, headers },
            DEVICE_CODE,
            DeviceTokenRequest,
            { database, crypto: environment.crypto },
        );

        // find the client's codes
        const now = environment.clock.now();
        const digest = await Digest.of(asked.device_code, environment.crypto);
        const [row] = await database
            .select()
            .from(deviceAuthorization.table)
            .where(eq(deviceAuthorization.table.deviceDigest, digest))
            .limit(1);
        if (row === undefined || row.clientId !== party.clientId) {
            throw OAuthError.of("INVALID_GRANT", "invalid device code");
        }
        // refuse lapsed codes
        else if (row.expiresAt <= now) {
            throw OAuthError.of("EXPIRED_TOKEN", "the device code expired");
        }

        // record the poll, slowing a device that polls too soon
        const target = deviceAuthorization.target(row);
        const isEarly = row.polledAt !== null && now - row.polledAt < row.interval;
        const interval = isEarly ? row.interval + INTERVAL_MILLISECONDS : row.interval;
        const decision = DeviceAuthorization.decision(row);
        await objects.mutate(objects.as(deviceAuthorization), {
            id: RequestId.create(environment),
            calls: [
                decision.status === "pending"
                    ? deviceAuthorization.call("poll", { ...target, polledAt: now, interval })
                    : deviceAuthorization.call("delete", target),
            ],
        });

        // answer a sign-in still pending
        if (decision.status === "pending") {
            throw isEarly
                ? OAuthError.of("SLOW_DOWN", "the device polls too often")
                : OAuthError.of("AUTHORIZATION_PENDING", "the person has yet to decide");
        }
        // refuse a denied sign-in
        else if (decision.status === "denied") {
            throw OAuthError.of("ACCESS_DENIED", "the person denied the device");
        }

        // issue the approving person's tokens under their consent, refusing one revoked since the approval
        const consent = await Snapshot.live(database)
            .read(oauthConsent.table)
            .findFirst({ where: { clientId: party.clientId, scope: decision.userId } });
        if (consent === undefined) {
            throw OAuthError.of("INVALID_GRANT", "the person revoked the device's consent");
        }
        const resource = row.resource ?? undefined;
        const authentication = AccountAuthentication.client(
            { userId: decision.userId, clientId: party.clientId },
            Number.POSITIVE_INFINITY,
            PackageResource.audience(resource),
            now,
        );

        return {
            authentication,
            clientId: party.clientId,
            scopes: row.scopes,
            ...(resource === undefined ? {} : { resource }),
            consent: { row: consent, family: row.id },
        };
    };
}

/** Draw half a user code. */
function userCodeHalf(environment: Pick<Environment, "random">): string {
    return Array.from({ length: USER_CODE_HALF }, () =>
        USER_CODE_ALPHABET.charAt(Random.int(environment.random, USER_CODE_ALPHABET.length)),
    ).join("");
}

/** Read a typed user code in the form it is stored by: upper case, without separators. */
function normalize(userCode: string): string {
    return userCode.toUpperCase().replaceAll(/[^A-Z]/gu, "");
}
