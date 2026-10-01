import { defineService } from "@destack/service";
import * as object from "../object/index.ts";
import { authentication } from "./authentication.ts";
import { directory } from "./directory/index.ts";
import { hostToken } from "./host.ts";

/** The object types the account service serves. */
export const accountObjects = {
    user: object.user,
    domain: object.domain,
    account: object.account,
    organisation: object.organisation,
    group: object.group,
    role: object.role,
    relationship: object.relationship,
    proposal: object.proposal,
    serviceAccount: object.serviceAccount,
    personalAccessToken: object.personalAccessToken,
    serviceToken: object.serviceToken,
    device: object.device,
    deviceKey: object.deviceKey,
    host: object.host,
    hostKey: object.hostKey,
    connection: object.connection,
    environment: object.environment,
    session: object.session,
    passkey: object.passkey,
    membership: object.membership,
    twoFactor: object.twoFactor,
    oauthClient: object.oauthClient,
    oauthConsent: object.oauthConsent,
    zone: object.zone,
} as const;

/** The account service. */
export const accountService = defineService("account", {
    authentication,

    // objects with sharing methods that grant, propose and explain access
    objects: accountObjects,

    // the universe's directory for cells to place zones and claim unique names
    directory,

    // hosts proving their keys for access tokens
    hostToken,
});
