import { serviceAccount } from "../access/service.ts";
import { defineRelationsPart } from "@destack/db";

import { installation } from "./installation.ts";
import { schedule } from "./schedule.ts";
import { spaceMigration } from "./migration.ts";

import { space } from "./space.ts";
import { deployment } from "./deployment.ts";
import { instance } from "./instance.ts";
import { spaceHost } from "./host.ts";
import { spaceSource, spaceRevision } from "./source.ts";
import { spaceTransfer } from "./transfer.ts";

/** Query relationships for space records. */
export const spaceRelations = defineRelationsPart(
    {
        spaceSource,
        spaceRevision,
        serviceAccount,
        deployment,
        instance,
        spaceHost,
        installation,
        schedule,
        space,
        spaceMigration,
        spaceTransfer,
    },
    (relation) => ({
        spaceTransfer: {
            space: relation.one.space({
                from: relation.spaceTransfer.spaceId,
                to: relation.space.id,
                optional: false,
            }),
        },
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
        spaceSource: {
            space: relation.one.space({
                from: relation.spaceSource.spaceId,
                to: relation.space.id,
                optional: false,
            }),
            applied: relation.one.spaceRevision({
                from: [relation.spaceSource.spaceId, relation.spaceSource.appliedRevisionId],
                to: [relation.spaceRevision.spaceId, relation.spaceRevision.id],
                optional: true,
            }),
        },
        spaceRevision: {
            space: relation.one.space({
                from: relation.spaceRevision.spaceId,
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
                from: [relation.spaceMigration.spaceId],
                to: [relation.space.id],
                optional: false,
            }),
        },
        space: {
            source: relation.one.spaceSource({
                from: relation.space.id,
                to: relation.spaceSource.spaceId,
                optional: true,
            }),
        },
    }),
);
