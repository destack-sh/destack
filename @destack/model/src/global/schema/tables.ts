import { account } from "../account/account.ts";
import { environment } from "../account/environment.ts";
import { accountSource, accountRevision } from "../account/source.ts";
import {
    organisation,
    organisationMembership,
    organisationInvitation,
} from "../account/organisation.ts";
import { personalAccessToken, personalAccessTokenPermission } from "../account/token.ts";
import {
    passkey,
    twoFactor,
    signingKey,
    authenticationReplay,
    oauthClient,
    oauthResource,
    oauthClientResource,
    oauthConsent,
    oauthAccessToken,
    oauthRefreshToken,
    oauthClientAssertion,
} from "../authentication/index.ts";
import { identity } from "../authentication/identity.ts";
import { accountMembership } from "../account/membership.ts";
import { roleBinding } from "../account/binding.ts";
import { accountInvitation } from "../account/invitation.ts";
import { group, groupMembership } from "../account/group.ts";
import { connectedAccount } from "../account/connection.ts";
import { role } from "../account/role.ts";
import { session } from "../authentication/session.ts";
import { rolePermission } from "../account/permission.ts";
import { deviceAuthorization } from "../authentication/device.ts";
import { user } from "../account/user.ts";
import { serviceAccount, serviceToken, serviceTokenPermission } from "../account/service.ts";
import { preference } from "../account/preference.ts";
import { tunnel } from "../host/tunnel.ts";
import { hostAccess } from "../host/access.ts";
import { region } from "../host/region.ts";
import { deviceKey } from "../host/key.ts";
import { device } from "../host/device.ts";
import { host } from "../host/host.ts";
import { route } from "../routing/route.ts";
import { domain } from "../routing/domain.ts";
import { space } from "../space/space.ts";
import { repository } from "../package/repository.ts";
import { packageTable } from "../package/package.ts";

/** Tables stored by global system administration. */
export const tables = {
    user,
    organisation,
    organisationMembership,
    organisationInvitation,
    account,
    environment,
    accountSource,
    accountRevision,
    accountMembership,
    accountInvitation,
    group,
    groupMembership,
    role,
    roleBinding,
    rolePermission,
    preference,
    connectedAccount,
    serviceAccount,
    serviceToken,
    serviceTokenPermission,
    personalAccessToken,
    personalAccessTokenPermission,

    identity,
    session,
    passkey,
    twoFactor,
    deviceAuthorization,
    signingKey,
    authenticationReplay,

    oauthClient,
    oauthResource,
    oauthClientResource,
    oauthConsent,
    oauthAccessToken,
    oauthRefreshToken,
    oauthClientAssertion,

    device,
    deviceKey,
    host,
    hostAccess,
    region,
    tunnel,

    space,
    repository,
    package: packageTable,
    domain,
    route,
};
