import { account } from "../account/account.ts";
import { signInRequest } from "../account/signin.ts";
import { roleBinding } from "../account/binding.ts";
import { connectedAccount } from "../account/connection.ts";
import { group, groupMembership } from "../account/group.ts";
import { identity } from "../account/identity.ts";
import { accountInvitation } from "../account/invitation.ts";
import { accountMembership } from "../account/membership.ts";
import { permission } from "../account/permission.ts";
import { preference } from "../account/preference.ts";
import { role } from "../account/role.ts";
import { session } from "../account/session.ts";
import { serviceAccount, serviceToken } from "../account/software.ts";
import { user } from "../account/user.ts";
import { hostAccess } from "../host/access.ts";
import { deviceKey } from "../host/key.ts";
import { device } from "../host/device.ts";
import { host } from "../host/host.ts";
import { region } from "../host/region.ts";
import { spaceRegistration } from "../host/registration.ts";
import { serverConnection } from "../host/server.ts";
import { tunnel } from "../host/tunnel.ts";
import { checkout } from "../package/checkout.ts";
import { packageTable } from "../package/package.ts";
import { repositoryReference } from "../package/reference.ts";
import { release } from "../package/release.ts";
import { repository } from "../package/repository.ts";
import { snapshot } from "../resource/snapshot.ts";
import { resourceBinding } from "../resource/binding.ts";
import { resourceMigration } from "../resource/migration.ts";
import { resource } from "../resource/resource.ts";
import { restoration } from "../resource/restoration.ts";
import { domain } from "../space/domain.ts";
import { installation } from "../space/installation.ts";
import { spaceMigration } from "../space/migration.ts";
import { route } from "../space/route.ts";
import { space } from "../space/space.ts";

/** Standard tables available to database compositions. */
export const tables = {
    account,
    signInRequest,
    roleBinding,
    connectedAccount,
    group,
    groupMembership,
    identity,
    accountInvitation,
    accountMembership,
    permission,
    preference,
    role,
    session,
    serviceAccount,
    serviceToken,
    user,
    checkout,
    packageTable,
    repositoryReference,
    release,
    repository,
    domain,
    installation,
    spaceMigration,
    route,
    space,
    hostAccess,
    deviceKey,
    device,
    host,
    region,
    spaceRegistration,
    serverConnection,
    tunnel,
    snapshot,
    resourceBinding,
    resourceMigration,
    resource,
    restoration,
};
