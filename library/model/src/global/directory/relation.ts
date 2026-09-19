import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { region } from "../host/region.ts";
import { spaceDirectory } from "./space.ts";
import { repositoryDirectory } from "./repository.ts";
import { packageDirectory } from "./package.ts";

/** Query globally reserved names and their regional locations. */
export const directoryRelations = defineRelationsPart({
    account,
    region,
    spaceDirectory,
    repositoryDirectory,
    packageDirectory,
}, (relation) => ({
    spaceDirectory: {
        account: relation.one.account({
            from: [relation.spaceDirectory.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        region: relation.one.region({
            from: [relation.spaceDirectory.regionId],
            to: [relation.region.id],
            optional: false,
        }),
    },
    repositoryDirectory: {
        account: relation.one.account({
            from: [relation.repositoryDirectory.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        region: relation.one.region({
            from: [relation.repositoryDirectory.regionId],
            to: [relation.region.id],
            optional: false,
        }),
    },
    packageDirectory: {
        repository: relation.one.repositoryDirectory({
            from: [relation.packageDirectory.accountId, relation.packageDirectory.repositoryId],
            to: [relation.repositoryDirectory.accountId, relation.repositoryDirectory.id],
            optional: false,
        }),
    },
}));
