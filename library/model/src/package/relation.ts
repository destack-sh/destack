import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { packageTable } from "../package/package.ts";
import { repositoryReference } from "../package/reference.ts";
import { release } from "../package/release.ts";
import { repository } from "../package/repository.ts";

/** Query relationships for package records. */
export const packageRelations = defineRelationsPart({
    repository,
    packageTable,
    account,
    repositoryReference,
    release,
}, (relation) => ({
    packageTable: {
        repository: relation.one.repository({
            from: [relation.packageTable.accountId, relation.packageTable.repositoryId],
            to: [relation.repository.accountId, relation.repository.id],
            optional: false,
        }),
        account: relation.one.account({
            from: [relation.packageTable.accountId],
            to: [relation.account.id],
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
    repository: {
        account: relation.one.account({
            from: [relation.repository.accountId],
            to: [relation.account.id],
            optional: false,
        }),
    },
}));
