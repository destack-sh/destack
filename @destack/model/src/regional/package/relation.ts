import { defineRelationsPart } from "@destack/db";

import { packageTable } from "./package.ts";
import { repositoryRef } from "./ref.ts";
import { release } from "./release.ts";
import { repository } from "./repository.ts";

/** Query relationships for package records. */
export const packageRelations = defineRelationsPart(
    {
        repository,
        package: packageTable,
        repositoryRef,
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
        repositoryRef: {
            repository: relation.one.repository({
                from: [relation.repositoryRef.repositoryId],
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
