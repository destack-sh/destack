import { deploymentSecretBinding, secret, secretBinding, secretVersion, vault } from "./vault.ts";
import { defineRelationsPart } from "@destack/db";

import { snapshot } from "./snapshot.ts";
import { resourceBinding } from "./binding.ts";
import { resourceMigration } from "./migration.ts";
import { resource } from "./resource.ts";
import { restoration } from "./restoration.ts";
import { installation } from "../space/installation.ts";
import { spaceMigration } from "../space/migration.ts";
import { space } from "../space/space.ts";
import { deployment } from "../space/deployment.ts";
import { deploymentBinding } from "./deployment.ts";

/** Query relationships for resource records. */
export const resourceRelations = defineRelationsPart({
    vault,
    secret,
    secretVersion,
    secretBinding,
    deploymentSecretBinding,
    deployment,
    deploymentBinding,
    snapshot,
    resource,
    installation,
    resourceBinding,
    space,
    resourceMigration,
    spaceMigration,
    restoration,
}, (relation) => ({
    vault: {
        resource: relation.one.resource({
            from: relation.vault.resourceId,
            to: relation.resource.id,
            optional: false,
        }),
        secrets: relation.many.secret({
            from: relation.vault.resourceId,
            to: relation.secret.vaultId,
        }),
    },
    secret: {
        vault: relation.one.vault({
            from: relation.secret.vaultId,
            to: relation.vault.resourceId,
            optional: false,
        }),
        versions: relation.many.secretVersion({
            from: relation.secret.id,
            to: relation.secretVersion.secretId,
        }),
        current: relation.one.secretVersion({
            from: [relation.secret.id, relation.secret.currentVersion],
            to: [relation.secretVersion.secretId, relation.secretVersion.version],
            optional: true,
        }),
    },
    secretVersion: {
        secret: relation.one.secret({
            from: relation.secretVersion.secretId,
            to: relation.secret.id,
            optional: false,
        }),
    },
    secretBinding: {
        installation: relation.one.installation({
            from: relation.secretBinding.installationId,
            to: relation.installation.id,
            optional: false,
        }),
        secret: relation.one.secret({
            from: relation.secretBinding.secretId,
            to: relation.secret.id,
            optional: false,
        }),
        secretVersion: relation.one.secretVersion({
            from: [relation.secretBinding.secretId, relation.secretBinding.version],
            to: [relation.secretVersion.secretId, relation.secretVersion.version],
            optional: true,
        }),
    },
    deploymentSecretBinding: {
        deployment: relation.one.deployment({
            from: relation.deploymentSecretBinding.deploymentId,
            to: relation.deployment.id,
            optional: false,
        }),
        secret: relation.one.secret({
            from: relation.deploymentSecretBinding.secretId,
            to: relation.secret.id,
            optional: false,
        }),
        secretVersion: relation.one.secretVersion({
            from: [
                relation.deploymentSecretBinding.secretId,
                relation.deploymentSecretBinding.version,
            ],
            to: [relation.secretVersion.secretId, relation.secretVersion.version],
            optional: true,
        }),
    },
    deploymentBinding: {
        deployment: relation.one.deployment({
            from: [relation.deploymentBinding.deploymentId],
            to: [relation.deployment.id],
            optional: false,
        }),
        resource: relation.one.resource({
            from: [relation.deploymentBinding.resourceId],
            to: [relation.resource.id],
            optional: false,
        }),
    },
    snapshot: {
        resource: relation.one.resource({
            from: [relation.snapshot.resourceId],
            to: [relation.resource.id],
            optional: false,
        }),
    },
    resourceBinding: {
        installation: relation.one.installation({
            from: [relation.resourceBinding.spaceId, relation.resourceBinding.installationId],
            to: [relation.installation.spaceId, relation.installation.id],
            optional: false,
        }),
        resource: relation.one.resource({
            from: [relation.resourceBinding.spaceId, relation.resourceBinding.resourceId],
            to: [relation.resource.spaceId, relation.resource.id],
            optional: false,
        }),
    },
    resourceMigration: {
        snapshot: relation.one.snapshot({
            from: [relation.resourceMigration.resourceId, relation.resourceMigration.snapshotId],
            to: [relation.snapshot.resourceId, relation.snapshot.id],
            optional: true,
        }),
        space: relation.one.space({
            from: [relation.resourceMigration.accountId, relation.resourceMigration.spaceId],
            to: [relation.space.accountId, relation.space.id],
            optional: false,
        }),
        spaceMigration: relation.one.spaceMigration({
            from: [relation.resourceMigration.spaceId, relation.resourceMigration.spaceMigrationId],
            to: [relation.spaceMigration.spaceId, relation.spaceMigration.id],
            optional: true,
        }),
        resource: relation.one.resource({
            from: [relation.resourceMigration.spaceId, relation.resourceMigration.resourceId],
            to: [relation.resource.spaceId, relation.resource.id],
            optional: false,
        }),
    },
    resource: {
        space: relation.one.space({
            from: [relation.resource.accountId, relation.resource.spaceId],
            to: [relation.space.accountId, relation.space.id],
            optional: false,
        }),
    },
    restoration: {
        snapshot: relation.one.snapshot({
            from: [relation.restoration.snapshotId],
            to: [relation.snapshot.id],
            optional: false,
        }),
        destinationResource: relation.one.resource({
            from: [relation.restoration.destinationResourceId],
            to: [relation.resource.id],
            optional: false,
        }),
    },
}));
