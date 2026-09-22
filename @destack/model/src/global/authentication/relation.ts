import { defineRelationsPart } from "@destack/db";
import { user } from "../account/user.ts";
import { account } from "../account/account.ts";
import { serviceAccount, serviceToken, serviceTokenPermission } from "../account/service.ts";
import { personalAccessToken, personalAccessTokenPermission } from "../account/token.ts";
import { device } from "../host/device.ts";
import { space } from "../space/space.ts";
import { identity } from "./identity.ts";
import { session } from "./session.ts";
import { deviceAuthorization } from "./device.ts";
import { passkey, twoFactor } from "./authenticator.ts";
import {
    oauthClient,
    oauthConsent,
    oauthRefreshToken,
    oauthAccessToken,
    oauthResource,
    oauthClientResource,
} from "./oauth/index.ts";

/** Query authentication credentials and the identities authorizing them. */
export const authenticationRelations = defineRelationsPart(
    {
        user,
        account,
        serviceAccount,
        serviceToken,
        serviceTokenPermission,
        device,
        space,
        oauthResource,
        identity,
        session,
        deviceAuthorization,
        passkey,
        twoFactor,
        oauthClient,
        oauthConsent,
        oauthRefreshToken,
        oauthAccessToken,
        oauthClientResource,
        personalAccessToken,
        personalAccessTokenPermission,
    },
    (relation) => ({
        identity: {
            user: relation.one.user({
                from: relation.identity.userId,
                to: relation.user.id,
                optional: false,
            }),
        },
        session: {
            user: relation.one.user({
                from: relation.session.userId,
                to: relation.user.id,
                optional: false,
            }),
            device: relation.one.device({
                from: relation.session.deviceId,
                to: relation.device.id,
                optional: true,
            }),
        },
        deviceAuthorization: {
            client: relation.one.oauthClient({
                from: relation.deviceAuthorization.oauthClientId,
                to: relation.oauthClient.clientId,
                optional: true,
            }),
            user: relation.one.user({
                from: relation.deviceAuthorization.userId,
                to: relation.user.id,
                optional: true,
            }),
        },
        passkey: {
            user: relation.one.user({
                from: relation.passkey.userId,
                to: relation.user.id,
                optional: false,
            }),
        },
        twoFactor: {
            user: relation.one.user({
                from: relation.twoFactor.userId,
                to: relation.user.id,
                optional: false,
            }),
        },
        oauthClient: {
            account: relation.one.account({
                from: relation.oauthClient.accountId,
                to: relation.account.id,
                optional: true,
            }),
            serviceAccount: relation.one.serviceAccount({
                from: [relation.oauthClient.accountId, relation.oauthClient.serviceAccountId],
                to: [relation.serviceAccount.accountId, relation.serviceAccount.id],
                optional: true,
            }),
            user: relation.one.user({
                from: relation.oauthClient.userId,
                to: relation.user.id,
                optional: true,
            }),
        },
        oauthConsent: {
            user: relation.one.user({
                from: relation.oauthConsent.userId,
                to: relation.user.id,
                optional: true,
            }),
            client: relation.one.oauthClient({
                from: relation.oauthConsent.clientId,
                to: relation.oauthClient.clientId,
                optional: false,
            }),
        },
        oauthRefreshToken: {
            user: relation.one.user({
                from: relation.oauthRefreshToken.userId,
                to: relation.user.id,
                optional: false,
            }),
            client: relation.one.oauthClient({
                from: relation.oauthRefreshToken.clientId,
                to: relation.oauthClient.clientId,
                optional: false,
            }),
            session: relation.one.session({
                from: relation.oauthRefreshToken.sessionId,
                to: relation.session.id,
                optional: true,
            }),
        },
        oauthAccessToken: {
            user: relation.one.user({
                from: relation.oauthAccessToken.userId,
                to: relation.user.id,
                optional: true,
            }),
            client: relation.one.oauthClient({
                from: relation.oauthAccessToken.clientId,
                to: relation.oauthClient.clientId,
                optional: false,
            }),
            session: relation.one.session({
                from: relation.oauthAccessToken.sessionId,
                to: relation.session.id,
                optional: true,
            }),
            refreshToken: relation.one.oauthRefreshToken({
                from: relation.oauthAccessToken.refreshId,
                to: relation.oauthRefreshToken.id,
                optional: true,
            }),
        },
        oauthClientResource: {
            client: relation.one.oauthClient({
                from: relation.oauthClientResource.clientId,
                to: relation.oauthClient.clientId,
                optional: false,
            }),
            resource: relation.one.oauthResource({
                from: relation.oauthClientResource.resourceId,
                to: relation.oauthResource.identifier,
                optional: false,
            }),
        },
        personalAccessToken: {
            user: relation.one.user({
                from: relation.personalAccessToken.userId,
                to: relation.user.id,
                optional: false,
            }),
            account: relation.one.account({
                from: relation.personalAccessToken.accountId,
                to: relation.account.id,
                optional: false,
            }),
        },
        personalAccessTokenPermission: {
            token: relation.one.personalAccessToken({
                from: relation.personalAccessTokenPermission.tokenId,
                to: relation.personalAccessToken.id,
                optional: false,
            }),
            space: relation.one.space({
                from: relation.personalAccessTokenPermission.spaceId,
                to: relation.space.id,
                optional: true,
            }),
        },
        serviceTokenPermission: {
            token: relation.one.serviceToken({
                from: relation.serviceTokenPermission.tokenId,
                to: relation.serviceToken.id,
                optional: false,
            }),
            space: relation.one.space({
                from: relation.serviceTokenPermission.spaceId,
                to: relation.space.id,
                optional: true,
            }),
        },
    }),
);
