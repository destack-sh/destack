import { defineRelationsPart } from "@destack/db";

import { packageTable } from "./package.ts";
import { repositoryReference } from "./reference.ts";
import { release } from "./release.ts";
import { repository } from "./repository.ts";
import { packagePolicy, packagePolicyRevision } from "./policy.ts";
import { space } from "../space/space.ts";

/** Query relationships for package records. */
export const packageRelations = defineRelationsPart(
    {
        repository,
        packageTable,
        repositoryReference,
        release,
        packagePolicy,
        packagePolicyRevision,
        space,
    },
    (relation) => ({
        packagePolicy: {
            space: relation.one.space({
                from: [relation.packagePolicy.accountId, relation.packagePolicy.spaceId],
                to: [relation.space.accountId, relation.space.id],
                optional: true,
            }),
            currentRevision: relation.one.packagePolicyRevision({
                from: [relation.packagePolicy.id, relation.packagePolicy.currentRevisionId],
                to: [relation.packagePolicyRevision.policyId, relation.packagePolicyRevision.id],
                optional: true,
            }),
        },
        packagePolicyRevision: {
            policy: relation.one.packagePolicy({
                from: [relation.packagePolicyRevision.policyId],
                to: [relation.packagePolicy.id],
                optional: false,
            }),
        },
        packageTable: {
            repository: relation.one.repository({
                from: [relation.packageTable.accountId, relation.packageTable.repositoryId],
                to: [relation.repository.accountId, relation.repository.id],
                optional: false,
            }),
        },
        repositoryReference: {
            repository: relation.one.repository({
                from: [relation.repositoryReference.repositoryId],
                to: [relation.repository.id],
                optional: false,
            }),
        },
        release: {
            package: relation.one.packageTable({
                from: [relation.release.packageId],
                to: [relation.packageTable.id],
                optional: false,
            }),
            repository: relation.one.repository({
                from: [relation.release.repositoryId],
                to: [relation.repository.id],
                optional: false,
            }),
        },
    }),
);
