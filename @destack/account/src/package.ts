import { role, universe } from "@destack/access";
import { defineMount, definePackage, proof } from "@destack/object";
import { AccountError } from "./error/error.ts";
import {
    account,
    entitlement,
    assignment,
    claim,
    client,
    domain,
    trustedPublisher,
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
import { OAUTH_PATH } from "./address/address.ts";

/** The account package: its objects, authentication records, OAuth mount and errors. */
export default definePackage({
    objects: [
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
        authorizationRequest,
        authorizationCode,
        refreshToken,
        deviceAuthorization,
        organization,
        account,
        stage,
        group,
        domain,
        trustedPublisher,
        hostname,
        machine,
        endpoint,
        identity,
        operation,
        residency,
        space,
        placement,
        assignment,
        claim,
        entitlement,
    ],
    tables: [
        // the authentication protocols' records
        verification,
        rateLimit,

        // the universe's identity keys
        identityKey,
    ],
    mounts: [defineMount({ name: "oauth", path: OAUTH_PATH })],
    errors: AccountError,
    // request what the platform's code does in the universe
    access: {
        permissions: {
            space: {
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
                    machine.permission("record"),
                    space.permission("upgrade"),
                    space.permission("receive"),
                ],
            },
        },
    },
});
