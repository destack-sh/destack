import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { space } from "../space/space.ts";
import { domain } from "./domain.ts";
import { route } from "./route.ts";

/** Query public routing records without joining regional application records. */
export const routingRelations = defineRelationsPart(
    { account, space, domain, route },
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
            space: relation.one.space({
                from: [relation.route.spaceId],
                to: [relation.space.id],
                optional: true,
            }),
        },
    }),
);
