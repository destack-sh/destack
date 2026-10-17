import { defineAdapter, definePackage } from "@destack/runtime";
import {
    account,
    entitlement,
    assignment,
    claim,
    client,
    domain,
    endpoint,
    environment,
    grant,
    group,
    hostname,
    identity,
    invitation,
    key,
    machine,
    member,
    oauthClient,
    oauthConsent,
    operation,
    organisation,
    passkey,
    placement,
    relationship,
    proof,
    residency,
    role,
    session,
    space,
    twoFactor,
    user,
    userLogin,
} from "./object/index.ts";

/** The path the oauth adapter answers below the account package's mount. */
export const OAUTH_PATH = "/oauth";

/** The account package: its objects and its OAuth adapter. */
export default definePackage({
    objects: {
        // people, their sign-ins and the programs they sign in through
        user,
        session,
        userLogin,
        passkey,
        twoFactor,
        client,
        key,
        oauthClient,
        oauthConsent,
        grant,

        // organisations and their members
        organisation,
        member,

        // accounts, their access, domains and machines
        account,
        environment,
        group,
        role,
        relationship,
        invitation,
        proof,
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
    adapters: { oauth: defineAdapter("oauth", { path: OAUTH_PATH }) },
});
