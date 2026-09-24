import { defineRelationsPart } from "@destack/db";

import { packageTable } from "./package.ts";
import { repositoryReference } from "./ref.ts";
import { release } from "./release.ts";
import { repository } from "./repository.ts";

/** Query relationships for package records. */
export const packageRelations = defineRelationsPart(
    {
        repository,
        package: packageTable,
        repositoryReference,
        release,
    },
    (relation) => ({
        package: {
            repository: relation.one.repository({
                from: [relation.package.accountId, relation.package.repositoryId],
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
            package: relation.one.package({
                from: [relation.release.packageId],
                to: [relation.package.id],
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
