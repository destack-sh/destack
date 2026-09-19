import { defineRelationsPart } from "@destack/db";

import { packageTable } from "./package.ts";
import { repositoryReference } from "./reference.ts";
import { release } from "./release.ts";
import { repository } from "./repository.ts";

/** Query relationships for package records. */
export const packageRelations = defineRelationsPart({
    repository,
    packageTable,
    repositoryReference,
    release,
}, (relation) => ({
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
}));
