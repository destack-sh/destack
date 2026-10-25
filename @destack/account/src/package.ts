import { role, universe } from "@destack/access";
import { defineAdapter, definePackage, proof } from "@destack/object";
import { AccountError } from "./error/error.ts";
import {
    account,
    entitlement,
    assignment,
    claim,
    client,
    domain,
    endpoint,
    stage,
    group,
    hostname,
    identity,
    key,
    machine,
    application,
    oauthConsent,
    authorizationRequest,
    authorizationCode,
    refreshToken,
    deviceAuthorization,
    operation,
    organization,
    passkey,
    placement,
    residency,
    session,
    space,
    challenge,
    factor,
    user,
    userLogin,
} from "./object/index.ts";
import { rateLimit, verification } from "./authentication/table.ts";
import { identityKey } from "./identity/key.ts";

/** The path the oauth adapter answers below the account package's mount. */
export const OAUTH_PATH = "/oauth";

/** The account package: its objects, authentication records, OAuth adapter and errors. */
export default definePackage({
    objects: {
        // people, their sign-ins and the programs they sign in through
        user,
        session,
        userLogin,
        passkey,
        challenge,
        factor,
        client,
        key,
        application,
        oauthConsent,

        // the authorization server's requests, codes, refresh tokens and device sign-ins
        authorizationRequest,
        authorizationCode,
        refreshToken,
        deviceAuthorization,

        // organizations
        organization,

        // accounts, their access, domains and machines
        account,
        stage,
        group,
        domain,
        hostname,
        machine,
        endpoint,

        // the universe's registry: identities, residencies, spaces, placements and names
        identity,
        operation,
        residency,
        space,
        placement,
        assignment,
        claim,
        entitlement,
    },
    tables: [
        // the authentication protocols' records
        verification,
        rateLimit,

        // the universe's identity keys
        identityKey,
    ],
    adapters: { oauth: defineAdapter("oauth", { path: OAUTH_PATH }) },
    errors: AccountError,
    // request what the platform's code does in the universe
    permissions: {
        installation: {
            universe: [
                role.permission("create"),
                role.permission("update"),
                role.permission("share"),
                universe.permission("grant"),
                proof.permission("spend"),
                user.permission("suspend"),
                account.permission("place"),
                organization.permission("place"),
                residency.permission("place"),
                organization.permission("administer"),
                machine.permission("rotate"),
                machine.permission("publish"),
                space.permission("upgrade"),
                space.permission("receive"),
            ],
        },
    },
});
