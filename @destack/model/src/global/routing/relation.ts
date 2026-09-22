import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { spaceDirectory } from "../directory/space.ts";
import { domain } from "./domain.ts";
import { route } from "./route.ts";

/** Query public routing records without joining regional application records. */
export const routingRelations = defineRelationsPart(
    { account, spaceDirectory, domain, route },
    (relation) => ({
        domain: {
            account: relation.one.account({
                from: [relation.domain.accountId],
                to: [relation.account.id],
                optional: false,
            }),
        },
        route: {
            domain: relation.one.domain({
                from: [relation.route.accountId, relation.route.domainId],
                to: [relation.domain.accountId, relation.domain.id],
                optional: false,
            }),
            space: relation.one.spaceDirectory({
                from: [relation.route.spaceId],
                to: [relation.spaceDirectory.id],
                optional: true,
            }),
        },
    }),
);
