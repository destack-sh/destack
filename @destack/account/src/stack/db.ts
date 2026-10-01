import type { Table } from "@destack/db";
import { journal } from "@destack/audit";
import { account } from "../object/account.ts";
import { group } from "../object/group.ts";
import { organisation } from "../object/organisation.ts";
import { serviceAccount } from "../object/service.ts";
import { personalAccessToken, serviceToken } from "../object/token.ts";
import { user } from "../object/user.ts";
import { domain } from "../object/domain.ts";
import { host, hostKey } from "../object/host.ts";
import { deviceAuthorization } from "./authentication/device.ts";
import { signingKey, authenticationReplay } from "./authentication/key.ts";
import {
    oauthResource,
    oauthClientResource,
    oauthClientAssertion,
} from "./authentication/oauth/authorization.ts";
import { oauthRefreshToken, oauthAccessToken } from "./authentication/oauth/token.ts";
import { device, deviceKey } from "../object/device.ts";
import { connection } from "../object/connection.ts";
import { environment } from "../object/environment.ts";
import { identity, passkey, session, twoFactor } from "../object/authentication.ts";
import { membership } from "../object/membership.ts";
import { oauthClient, oauthConsent } from "../object/oauth.ts";
import { region } from "../object/region.ts";
import { verification, rateLimit } from "./verification.ts";

/** The account service's tables: accounts, sign-in and the OpenID provider. */
export const accountTables: readonly Table[] = [
    // accounts, organisations, tokens, devices and connections
    ...account.tables,
    ...connection.tables,
    ...environment.tables,
    ...group.tables,
    ...organisation.tables,
    ...serviceAccount.tables,
    ...personalAccessToken.tables,
    ...serviceToken.tables,
    ...device.tables,
    ...deviceKey.tables,
    ...region.tables,
    ...domain.tables,
    ...host.tables,
    ...hostKey.tables,
    journal,

    // sign-in
    ...user.tables,
    ...session.tables,
    ...identity.tables,
    ...passkey.tables,
    ...membership.tables,
    ...twoFactor.tables,
    deviceAuthorization,
    signingKey,
    authenticationReplay,
    verification,
    rateLimit,

    // the OpenID Connect provider signing users in to other applications
    ...oauthClient.tables,
    ...oauthConsent.tables,
    oauthResource,
    oauthClientResource,
    oauthClientAssertion,
    oauthRefreshToken,
    oauthAccessToken,
];
