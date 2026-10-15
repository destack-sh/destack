import type { OAuthExtensionGrantHandler } from "@better-auth/oauth-provider";
import { principal } from "@destack/access";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { type Clock, present, schema } from "@destack/schema";
import { isCallError } from "@destack/runtime/error";
import { isAPIError } from "better-auth/api";
import { Scope } from "@destack/sync";
import { Client } from "../object/client.ts";
import { user } from "../object/user.ts";
import { Issuance } from "./issuance.ts";
import { PackageResource } from "./resource.ts";
import { SignUp } from "../better-auth/sign-up.ts";

/** The grant type of a mailed sign-in code exchanged for a person's tokens on a new client. */
export const EMAIL_CODE = "urn:destack:grant-type:email-code";

/** The scopes an email code grants: the person's ID token and profile, and a refresh token. */
const CODE_SCOPES = ["openid", "profile", "offline_access"];

/** An email code request: the email, code, client and package, and a new person's sign-up. */
const CodeRequest = schema.looseObject({
    /** The email the code was mailed to. */
    email: schema.email(),
    /** The mailed code. */
    code: schema.string().min(1),
    /** The device's new client the tokens are issued to. */
    client_id: schema.identifier("client"),
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

/** What the email code grant checks codes with and registers clients in. */
export interface CodeOptions {
    /** The account database with the people and their clients. */
    readonly database: DatabaseConnection;
    /** The account objects, which register the device's client for the person. */
    readonly objects: Pick<ObjectStore, "mutate" | "database">;
    /** The check of mailed sign-in codes. */
    readonly check: CodeCheck;
    /** The clock tokens are issued at. */
    readonly clock: Clock;
    /** The lifetimes of the packages' tokens, a minute when absent. */
    readonly lifetimes: ReadonlyMap<PackageId, number>;
}

/** Exchange a mailed sign-in code for a person's tokens on a device's new client. */
export function codeGrant(options: CodeOptions): OAuthExtensionGrantHandler {
    return async (input) => {
        // read the request and the package it asks a token for
        const request = CodeRequest.safeParse(input.ctx.body);
        if (!request.success) {
            throw Issuance.refusal("invalid_request", "invalid email code request");
        }
        const { email, code, client_id: clientId, resource } = request.data;
        const audience = PackageResource.package(resource);
        if (audience === undefined) {
            throw Issuance.refusal("invalid_target", `${resource} names no package`);
        }

        // spend the code, refusing a wrong or expired one
        await options.check(email, code).catch((error: unknown) => {
            throw isAPIError(error)
                ? Issuance.refusal("invalid_grant", "the code is wrong or expired")
                : error;
        });

        // read the person, or sign a new one up with the request's handle
        const found = await Snapshot.live(options.database)
            .read(user.table)
            .findFirst({
                where: {
                    email: email.toLowerCase(),
                },
            });
        const person = found ?? (await signUp(input, request.data, options.database));

        // leave a person with a second factor to the browser
        if (person.twoFactorEnabled === true) {
            throw Issuance.refusal(
                "invalid_grant",
                "a person with a second factor signs in in the browser",
            );
        }

        // prove the email, and register the device's client for the person
        const now = options.clock.now();
        const userId = person.id;
        if (person.emailVerified !== true) {
            await input.ctx.context.internalAdapter.updateUser(userId, { emailVerified: true });
        }
        if (!(await Client.isAdmissible(options.database, clientId))) {
            throw Issuance.refusal("invalid_grant", `${clientId} is revoked`);
        }
        await Client.register(options.objects, clientId, userId, now).catch((error: unknown) => {
            throw isCallError(error) ? Issuance.refusal("invalid_grant", error.message) : error;
        });

        // sign the person in on the client with a session the code proved, which its refresh tokens belong to
        const { internalAdapter } = input.ctx.context;
        const signedIn = await internalAdapter.createSession(userId);
        await internalAdapter.updateSession(signedIn.token, {
            authenticatedAt: now,
            authenticationMethod: "email-otp",
            clientId,
        });

        // issue the person's tokens to the client
        const subject = principal.user.reference(Scope.universe.id, userId);
        const caller = {
            subject,
            subjects: [subject],
            assurance: { level: 1, authenticatedAt: now },
            credential: {
                kind: "client",
                id: clientId,
                client: clientId,
                audience,
                verifiedAt: now,
                expiresAt: now,
            },
        };

        return Issuance.issue(input, caller, {
            now,
            lifetimes: options.lifetimes,
            scopes: CODE_SCOPES,
            session: signedIn.id,
        });
    };
}

/** Sign a new person up with a code request's handle, residency, name, time zone and locale. */
async function signUp(
    input: Parameters<OAuthExtensionGrantHandler>[0],
    request: schema.Infer<typeof CodeRequest>,
    database: DatabaseConnection,
) {
    // refuse a new email without a handle
    if (request.handle === undefined) {
        throw Issuance.refusal(
            "invalid_grant",
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
        throw Issuance.refusal(
            "invalid_request",
            "a sign-up needs a name, a handle, a residency, a time zone and a locale",
        );
    }

    // create the person and their personal account as the browser's sign-up does, the code proving the email
    const created = await input.ctx.context.internalAdapter.createUser(
        { email: request.email.toLowerCase(), emailVerified: true, ...values.data },
        input.ctx,
    );
    const person = await Snapshot.live(database)
        .read(user.table)
        .findFirst({
            where: {
                id: user.identifier(created.id),
            },
        });

    return present(person, `the signed-up person ${created.id}`);
}
