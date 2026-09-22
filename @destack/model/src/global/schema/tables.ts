import { account } from "../account/account.ts";
import { identity } from "../authentication/identity.ts";
import { accountMembership } from "../account/membership.ts";
import { roleBinding } from "../account/binding.ts";
import { accountInvitation } from "../account/invitation.ts";
import { group, groupMembership } from "../account/group.ts";
import { connectedAccount } from "../account/connection.ts";
import { role } from "../account/role.ts";
import { session } from "../authentication/session.ts";
import { rolePermission } from "../account/permission.ts";
import { signInRequest } from "../authentication/signin.ts";
import { user } from "../account/user.ts";
import { serviceAccount, serviceToken } from "../account/service.ts";
import { preference } from "../account/preference.ts";
import { tunnel } from "../host/tunnel.ts";
import { hostAccess } from "../host/access.ts";
import { region } from "../host/region.ts";
import { deviceKey } from "../host/key.ts";
import { device } from "../host/device.ts";
import { host } from "../host/host.ts";
import { route } from "../routing/route.ts";
import { domain } from "../routing/domain.ts";
import { spaceDirectory } from "../directory/space.ts";
import { repositoryDirectory } from "../directory/repository.ts";
import { packageDirectory } from "../directory/package.ts";

/** Tables stored by global system administration. */
export const tables = {
    account,
    identity,
    accountMembership,
    roleBinding,
    accountInvitation,
    group,
    groupMembership,
    connectedAccount,
    role,
    session,
    rolePermission,
    signInRequest,
    user,
    serviceAccount,
    serviceToken,
    preference,
    tunnel,
    hostAccess,
    region,
    deviceKey,
    device,
    host,
    route,
    domain,
    spaceDirectory,
    repositoryDirectory,
    packageDirectory,
};
