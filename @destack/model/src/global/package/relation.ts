import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { region } from "../host/region.ts";
import { repository } from "./repository.ts";
import { packageTable } from "./package.ts";

/** Query globally named repositories and packages. */
export const packageRelations = defineRelationsPart(
    { repository, package: packageTable, account, region },
    (relation) => ({
        repository: {
            account: relation.one.account({
                from: relation.repository.accountId,
                to: relation.account.id,
                optional: false,
            }),
            region: relation.one.region({
                from: relation.repository.regionId,
                to: relation.region.id,
                optional: false,
            }),
        },
        package: {
            repository: relation.one.repository({
                from: [relation.package.accountId, relation.package.repositoryId],
                to: [relation.repository.accountId, relation.repository.id],
                optional: false,
            }),
        },
    }),
);
