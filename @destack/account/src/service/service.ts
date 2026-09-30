import { defineService } from "@destack/service";
import * as object from "../object/index.ts";
import { authentication } from "./authentication.ts";
import { directory } from "./directory/index.ts";

/** The object types the account service serves. */
export const accountObjects = {
    user: object.user,
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

    // the universe's copies for the cells of spaces and the hosts of accounts

    // the universe's directory for cells to place zones and claim unique names
    directory,
});
