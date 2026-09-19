import { serviceAccount } from "../access/service.ts";
import { defineRelationsPart } from "@destack/db";

import { installation } from "./installation.ts";
import { schedule } from "./schedule.ts";
import { spaceMigration } from "./migration.ts";

import { space } from "./space.ts";
import { deployment } from "./deployment.ts";
import { instance } from "./instance.ts";
import { spaceHost } from "./host.ts";
import { spaceConfiguration, stackRevision } from "./configuration.ts";

/** Query relationships for space records. */
export const spaceRelations = defineRelationsPart({
    spaceConfiguration,
    stackRevision,
    serviceAccount,
    deployment,
    instance,
    spaceHost,
    installation,
    schedule,
    space,
    spaceMigration,
}, (relation) => ({
    schedule: {
        installation: relation.one.installation({
            from: relation.schedule.installationId,
            to: relation.installation.id,
            optional: false,
        }),
        serviceAccount: relation.one.serviceAccount({
            from: relation.schedule.serviceAccountId,
            to: relation.serviceAccount.id,
            optional: false,
        }),
    },
    spaceConfiguration: {
        space: relation.one.space({
            from: relation.spaceConfiguration.spaceId,
            to: relation.space.id,
            optional: false,
        }),
        applied: relation.one.stackRevision({
            from: [
                relation.spaceConfiguration.spaceId,
                relation.spaceConfiguration.appliedRevisionId,
            ],
            to: [relation.stackRevision.spaceId, relation.stackRevision.id],
            optional: true,
        }),
    },
    stackRevision: {
        space: relation.one.space({
            from: relation.stackRevision.spaceId,
            to: relation.space.id,
            optional: false,
        }),
    },
    deployment: {
        serviceAccount: relation.one.serviceAccount({
            from: relation.deployment.serviceAccountId,
            to: relation.serviceAccount.id,
            optional: false,
        }),
        installation: relation.one.installation({
            from: [relation.deployment.installationId],
            to: [relation.installation.id],
            optional: false,
        }),
        host: relation.one.spaceHost({
            from: [relation.deployment.spaceId, relation.deployment.hostId],
            to: [relation.spaceHost.spaceId, relation.spaceHost.hostId],
            optional: true,
        }),
    },
    instance: {
        deployment: relation.one.deployment({
            from: [relation.instance.deploymentId],
            to: [relation.deployment.id],
            optional: false,
        }),
        host: relation.one.spaceHost({
            from: [relation.instance.spaceId, relation.instance.hostId],
            to: [relation.spaceHost.spaceId, relation.spaceHost.hostId],
            optional: false,
        }),
    },
    spaceHost: {
        space: relation.one.space({
            from: [relation.spaceHost.spaceId],
            to: [relation.space.id],
            optional: false,
        }),
    },
    installation: {
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
    },
    space: {
        configuration: relation.one.spaceConfiguration({
            from: relation.space.id,
            to: relation.spaceConfiguration.spaceId,
            optional: true,
        }),
    },
}));
