import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { environment } from "../account/environment.ts";
import { region } from "../host/region.ts";
import { host } from "../host/host.ts";
import { space } from "./space.ts";

/** Query a space's global owner and administration region. */
export const spaceRelations = defineRelationsPart(
    { space, account, region, host, environment },
    (relation) => ({
        space: {
            authorityHost: relation.one.host({
                from: [relation.space.accountId, relation.space.authorityHostId],
                to: [relation.host.accountId, relation.host.id],
                optional: true,
            }),
            environment: relation.one.environment({
                from: [relation.space.accountId, relation.space.environmentId],
                to: [relation.environment.accountId, relation.environment.id],
                optional: true,
            }),
            account: relation.one.account({
                from: relation.space.accountId,
                to: relation.account.id,
                optional: false,
            }),
            region: relation.one.region({
                from: relation.space.regionId,
                to: relation.region.id,
                optional: false,
            }),
        },
    }),
);
