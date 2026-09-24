import { repository } from "../package/repository.ts";
import { repositoryReference } from "../package/ref.ts";
import { release } from "../package/release.ts";
import { packageTable } from "../package/package.ts";

import { spaceMigration } from "../space/migration.ts";
import { spaceTransfer } from "../space/transfer.ts";
import { space } from "../space/space.ts";
import { installation } from "../space/installation.ts";
import { spaceHost } from "../space/host.ts";
import { deployment } from "../space/deployment.ts";
import { instance } from "../space/instance.ts";
import { spaceSource, spaceRevision } from "../space/source.ts";
import { restoration } from "../resource/restoration.ts";
import { snapshot } from "../resource/snapshot.ts";
import { resourceBinding } from "../resource/binding.ts";
import { resourceMigration } from "../resource/migration.ts";
import {
    deploymentSecretBinding,
    secret,
    secretBinding,
    secretVersion,
    vault,
} from "../resource/vault.ts";
import { resource } from "../resource/resource.ts";
import { deploymentBinding } from "../resource/deployment.ts";
import { packagePolicy, packagePolicyRevision } from "../access/package.ts";
import { role } from "../access/role.ts";
import { roleBinding } from "../access/binding.ts";
import { rolePermission } from "../access/permission.ts";
import { serviceAccount, serviceToken } from "../access/service.ts";
import { schedule } from "../space/schedule.ts";
import { networkPolicy, networkPolicyRevision } from "../access/network.ts";

/** Tables managed by regional administration and package services. */
export const tables = {
    repository,
    repositoryReference,
    release,
    package: packageTable,
    spaceTransfer,
    spaceMigration,
    space,
    installation,
    spaceHost,
    deployment,
    instance,
    spaceSource,
    spaceRevision,
    restoration,
    snapshot,
    resourceBinding,
    resourceMigration,
    vault,
    secret,
    secretVersion,
    secretBinding,
    deploymentSecretBinding,
    resource,
    deploymentBinding,
    packagePolicy,
    packagePolicyRevision,
    role,
    roleBinding,
    rolePermission,
    serviceAccount,
    serviceToken,
    schedule,
    networkPolicy,
    networkPolicyRevision,
};
