import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { user } from "../account/user.ts";
import { hostAccess } from "../host/access.ts";
import { release } from "../package/release.ts";
import { domain } from "../space/domain.ts";
import { installation } from "../space/installation.ts";
import { spaceMigration } from "../space/migration.ts";
import { route } from "../space/route.ts";
import { space } from "../space/space.ts";

/** Query relationships for space records. */
export const spaceRelations = defineRelationsPart({
    account,
    domain,
    release,
    installation,
    space,
    spaceMigration,
    hostAccess,
    user,
    route,
}, (relation) => ({
    domain: {
        account: relation.one.account({
            from: [relation.domain.accountId],
            to: [relation.account.id],
            optional: false,
        }),
    },
    installation: {
        release: relation.one.release({
            from: [relation.installation.packageId, relation.installation.version],
            to: [relation.release.packageId, relation.release.version],
            optional: false,
        }),
        appliedRelease: relation.one.release({
            from: [relation.installation.packageId, relation.installation.appliedVersion],
            to: [relation.release.packageId, relation.release.version],
            optional: true,
        }),
        space: relation.one.space({
            from: [relation.installation.spaceId],
            to: [relation.space.id],
            optional: false,
        }),
    },
    spaceMigration: {
        space: relation.one.space({
            from: [relation.spaceMigration.accountId, relation.spaceMigration.spaceId],
            to: [relation.space.accountId, relation.space.id],
            optional: false,
        }),
        sourceHost: relation.one.hostAccess({
            from: [relation.spaceMigration.accountId, relation.spaceMigration.sourceHostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: false,
        }),
        targetHost: relation.one.hostAccess({
            from: [relation.spaceMigration.accountId, relation.spaceMigration.targetHostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: false,
        }),
        account: relation.one.account({
            from: [relation.spaceMigration.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        requester: relation.one.user({
            from: [relation.spaceMigration.requestedBy],
            to: [relation.user.id],
            optional: true,
        }),
    },
    route: {
        space: relation.one.space({
            from: [relation.route.accountId, relation.route.spaceId],
            to: [relation.space.accountId, relation.space.id],
            optional: true,
        }),
        installation: relation.one.installation({
            from: [relation.route.spaceId, relation.route.installationId],
            to: [relation.installation.spaceId, relation.installation.id],
            optional: true,
        }),
        account: relation.one.account({
            from: [relation.route.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        domain: relation.one.domain({
            from: [relation.route.domainId],
            to: [relation.domain.id],
            optional: false,
        }),
    },
    space: {
        host: relation.one.hostAccess({
            from: [relation.space.accountId, relation.space.hostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: true,
        }),
        requestedHost: relation.one.hostAccess({
            from: [relation.space.accountId, relation.space.requestedHostId],
            to: [relation.hostAccess.accountId, relation.hostAccess.hostId],
            optional: true,
        }),
        account: relation.one.account({
            from: [relation.space.accountId],
            to: [relation.account.id],
            optional: false,
        }),
    },
}));
