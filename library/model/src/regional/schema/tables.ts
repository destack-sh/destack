import { spaceMigration } from "../space/migration.ts";
import { space } from "../space/space.ts";
import { installation } from "../space/installation.ts";
import { spaceHost } from "../space/host.ts";
import { deployment } from "../space/deployment.ts";
import { instance } from "../space/instance.ts";
import { spaceConfiguration, stackRevision } from "../space/configuration.ts";
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
import { repository } from "../package/repository.ts";
import { repositoryReference } from "../package/reference.ts";
import { release } from "../package/release.ts";
import { packageTable } from "../package/package.ts";
import { role } from "../access/role.ts";
import { roleBinding } from "../access/binding.ts";
import { rolePermission } from "../access/permission.ts";
import { serviceAccount, serviceToken } from "../access/service.ts";
import { schedule } from "../space/schedule.ts";

/** Tables stored by regional system administration. */
export const tables = {
    spaceMigration,
    space,
    installation,
    spaceHost,
    deployment,
    instance,
    spaceConfiguration,
    stackRevision,
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
    repository,
    repositoryReference,
    release,
    packageTable,
    role,
    roleBinding,
    rolePermission,
    serviceAccount,
    serviceToken,
    schedule,
};
