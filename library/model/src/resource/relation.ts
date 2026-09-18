import { defineRelationsPart } from "@destack/db";
import { hostAccess } from "../host/access.ts";
import { region } from "../host/region.ts";
import { release } from "../package/release.ts";
import { snapshot } from "../resource/snapshot.ts";
import { resourceBinding } from "../resource/binding.ts";
import { resourceMigration } from "../resource/migration.ts";
import { resource } from "../resource/resource.ts";
import { restoration } from "../resource/restoration.ts";
import { installation } from "../space/installation.ts";
import { spaceMigration } from "../space/migration.ts";
import { space } from "../space/space.ts";

/** Query relationships for resource records. */
export const resourceRelations = defineRelationsPart({
    region,
    snapshot,
    resource,
    installation,
    resourceBinding,
    space,
    resourceMigration,
    hostAccess,
    spaceMigration,
    release,
    restoration,
}, (relation) => ({
    snapshot: {
        region: relation.one.region({
            from: [relation.snapshot.providerCode, relation.snapshot.regionId],
            to: [relation.region.providerCode, relation.region.id],
            optional: true,
        }),
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
        sourceHost: relation.one.hostAccess({
            from: [relation.resourceMigration.accountId, relation.resourceMigration.sourceHostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: true,
        }),
        targetHost: relation.one.hostAccess({
            from: [relation.resourceMigration.accountId, relation.resourceMigration.targetHostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: true,
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
        definitionRelease: relation.one.release({
            from: [relation.resource.definitionPackageId, relation.resource.definitionVersion],
            to: [relation.release.packageId, relation.release.version],
            optional: false,
        }),
        region: relation.one.region({
            from: [relation.resource.providerCode, relation.resource.regionId],
            to: [relation.region.providerCode, relation.region.id],
            optional: true,
        }),
        requestedRegion: relation.one.region({
            from: [relation.resource.requestedProviderCode, relation.resource.requestedRegionId],
            to: [relation.region.providerCode, relation.region.id],
            optional: true,
        }),
        space: relation.one.space({
            from: [relation.resource.accountId, relation.resource.spaceId],
            to: [relation.space.accountId, relation.space.id],
            optional: false,
        }),
        host: relation.one.hostAccess({
            from: [relation.resource.accountId, relation.resource.hostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: true,
        }),
        requestedHost: relation.one.hostAccess({
            from: [relation.resource.accountId, relation.resource.requestedHostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: true,
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
