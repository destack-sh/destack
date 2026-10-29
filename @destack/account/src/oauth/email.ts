import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { Environment } from "@destack/environment";
import type { ObjectStore } from "@destack/object/server";
import { present, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import type { AuthContext } from "better-auth";
import { isAPIError } from "better-auth/api";
import { OAuthError, SECOND_MILLISECONDS } from "../authentication/token.ts";
import { AccountAuthentication } from "../better-auth/authentication.ts";
import { SignUp } from "../better-auth/sign-up.ts";
import { Client } from "../object/client.ts";
import { factor } from "../object/factor.ts";
import { OAuthConsent } from "../object/oauth-consent.ts";
import { session } from "../object/session.ts";
import { user } from "../object/user.ts";
import { ClientAuthentication, EMAIL_CODE } from "./party.ts";
import { PackageResource } from "./resource.ts";
import { OFFLINE_ACCESS, OPENID, type TokenGrant } from "./token.ts";

/** The scopes an email code grants: the person's ID token and profile, and a refresh token. */
const CODE_SCOPES = [OPENID, "profile", OFFLINE_ACCESS];

/** An email code request: the email, code and package, and a new person's sign-up. */
const CodeRequest = schema.looseObject({
    /** The email the code was mailed to. */
    email: schema.email(),
    /** The mailed code. */
    code: schema.string().min(1),
    /** The package the token is for, as its resource (RFC 8707). */
    resource: schema.string().min(1),
    /** A new person's display name. */
    name: schema.string().exactOptional(),
    /** A new person's handle, which signs them up. */
    handle: schema.string().exactOptional(),
    /** A new person's residency. */
    residency_id: schema.string().exactOptional(),
    /** A new person's time zone. */
    time_zone: schema.string().exactOptional(),
    /** A new person's locale. */
    locale: schema.string().exactOptional(),
});

/** Check a mailed sign-in code of an email and spend it, refusing a wrong or expired one. */
export type CodeCheck = (email: string, code: string) => Promise<void>;

/** What the email code grant checks codes with and signs people in with. */
export interface EmailOptions {
    /** The account database with the people and their clients. */
    readonly database: DatabaseConnection;
    /** The account objects, which register the device's client and its consent. */
    readonly objects: Pick<ObjectStore, "mutate" | "as" | "database" | "environment">;
    /** The check of mailed sign-in codes. */
    readonly codes: CodeCheck;
    /** The Better Auth context, which creates the people and sessions the codes sign in. */
    readonly context: Promise<Pick<AuthContext, "internalAdapter" | "sessionConfig">>;
    /** The clock tokens are issued at, and the WebCrypto. */
    readonly environment: Pick<Environment, "clock" | "crypto">;
}

/** Exchange a mailed sign-in code for a person's tokens on a device's new client. */
export function emailGrant(options: EmailOptions): TokenGrant {
    return async ({ body, headers }) => {
        // identify the device's client and read the request
        const { database, environment } = options;
        const { party, asked } = await ClientAuthentication.grant(
            { body, headers },
            EMAIL_CODE,
            CodeRequest,
            { database, crypto: environment.crypto },
        );
        const { email, code, resource } = asked;
        const audience = PackageResource.package(resource);
        if (audience === undefined) {
            throw OAuthError.of("INVALID_TARGET", `${resource} names no package`);
        }

        // spend the code, refusing a wrong or expired one
        await options.codes(email, code).catch((error: unknown) => {
            throw isAPIError(error)
                ? OAuthError.of("INVALID_GRANT", "the code is wrong or expired")
                : error;
        });

        // read the person, or sign a new one up with the request's handle
        const context = await options.context;
        const found = await Snapshot.live(database)
            .read(user.table)
            .findFirst({ where: { email: email.toLowerCase() } });
        const person = found ?? (await signUp(context, asked, database));

        // leave a person with a second factor to the browser
        const verified = await Snapshot.live(database)
            .read(factor.table)
            .findFirst({ where: { userId: person.id, status: "verified" } });
        if (verified !== undefined) {
            throw OAuthError.of(
                "INVALID_GRANT",
                "a person with a second factor signs in in the browser",
            );
        }

        // prove the email, register the device's client and grant it the code's scopes
        const now = environment.clock.now();
        const userId = person.id;
        const { internalAdapter } = context;
        if (person.emailVerified !== true) {
            await internalAdapter.updateUser(userId, { emailVerified: true });
        }
        await Client.register(options.objects, party.clientId, userId, now);
        const consent = await OAuthConsent.grant(
            options.objects,
            { clientId: party.clientId, userId, scopes: CODE_SCOPES },
            now,
        );

        // sign the person in on the client with a session the code proved, timed on the store's clock
        const lifetime = context.sessionConfig.expiresIn * SECOND_MILLISECONDS;
        const signedIn = await internalAdapter.createSession(
            userId,
            false,
            {
                authenticatedAt: now,
                authenticationMethod: "email-otp",
                createdAt: new Date(now),
                updatedAt: new Date(now),
                expiresAt: new Date(now + lifetime),
            },
            true,
        );
        await options.objects.mutate(options.objects.as(session), {
            id: RequestId.create(options.objects.environment),
            calls: [
                session.call("link", {
                    ...session.target({ scope: userId, id: signedIn.id }),
                    clientId: party.clientId,
                }),
            ],
        });

        // describe the person through the session the code proved
        const authentication = await AccountAuthentication.person(
            database,
            { userId, clientId: party.clientId, sessionId: signedIn.id },
            audience,
            now,
        );

        return {
            authentication,
            clientId: party.clientId,
            scopes: CODE_SCOPES,
            resource,
            consent: { row: consent, family: signedIn.id, sessionId: signedIn.id },
        };
    };
}

/** Sign a new person up with a code request's handle, residency, name, time zone and locale. */
async function signUp(
    context: Pick<AuthContext, "internalAdapter">,
    request: schema.Infer<typeof CodeRequest>,
    database: DatabaseConnection,
) {
    // refuse a new email without a handle
    if (request.handle === undefined) {
        throw OAuthError.of(
            "INVALID_GRANT",
            `nobody signed up as ${request.email}: sign up with a handle`,
        );
    }
    const values = SignUp.safeParse({
        name: request.name,
        handle: request.handle,
        residencyId: request.residency_id,
        timeZone: request.time_zone,
        locale: request.locale,
    });
    if (!values.success) {
        throw OAuthError.of(
            "INVALID_REQUEST",
            "a sign-up needs a name, a handle, a residency, a time zone and a locale",
        );
    }

    // create the person and their personal account as the browser's sign-up does, the code proving the email
    const created = await context.internalAdapter.createUser(
        { email: request.email.toLowerCase(), emailVerified: true, ...values.data },
        { method: "email-otp" },
    );
    const person = await Snapshot.live(database)
        .read(user.table)
        .findFirst({ where: { id: user.identifier(created.id) } });

    return present(person, `the signed-up person ${created.id}`);
}
