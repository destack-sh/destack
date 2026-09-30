import { principal } from "@destack/access";
import { Condition } from "@destack/db/query";
import { identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { Scope, type QueryPage } from "@destack/sync";
import { expect, test } from "@destack/test";
import { v7 } from "uuid";
import { DirectoryClient } from "../src/client/index.ts";
import { account } from "../src/object/index.ts";
import { AccountFixture, type Host } from "./fixture.ts";

test("stream a space's cell the users who joined the space and their personal accounts with their public fields, and no others", async () => {
    // place a space of the owner's account in a host, and sign in a stranger and a person who joins the space
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    await fixture.signIn("stranger@example.com");
    const joiner = await fixture.signIn("joiner@example.com");
    const personal = await fixture.createSpace(joiner);
    const host = fixture.host(accountId);
    const spaceId = identifier("space").parse(`space-${v7()}`);
    await new DirectoryClient(host.client).place({
        id: spaceId,
        scope: accountId,
        cell: host.id,
        epoch: 1,
    });
    await joiner.client.membership.create({
        userId: joiner.id,
        requestId: RequestId.create(),
        spaceId,
    });

    // follow every user the space may read, receiving the joiner alone with its private fields concealed
    const concealed = [
        "email",
        "emailVerified",
        "login",
        "locale",
        "timeZone",
        "residency",
        "home",
        "twoFactorEnabled",
        "suspendedAt",
    ];
    expect(await follow(host, spaceId)).toEqual([
        ["insert", joiner.id, "joiner@example.com", concealed],
        [
            "insert",
            personal.accountId,
            personal.handle,
            [
                "defaultResidency",
                "packagePolicyId",
                "packagePolicyRegion",
                "networkPolicyId",
                "networkPolicyRegion",
            ],
        ],
    ]);

    // leave the space, after which the joiner is no longer streamed
    const { items } = await joiner.client.membership.list({ userId: joiner.id, limit: 10 });
    await joiner.client.membership.delete({
        userId: joiner.id,
        requestId: RequestId.create(),
        id: items[0]!.id,
        revision: items[0]!.revision,
    });
    expect(await follow(host, spaceId)).toEqual([]);
});

/** Read the user and account rows of the first page a space's cell follows. */
async function follow(host: Host, spaceId: string) {
    // ask for the users and the accounts in their scopes as the space's copy of the universe
    const controller = new AbortController();
    const pages = await host.client.replica.stream(
        {
            name: spaceId,
            scope: Scope.universe.id,
            below: spaceId,
            access: false,
            held: [],
            copied: [],
            rows: [
                {
                    type: {
                        packageId: principal.user.definition.packageId,
                        type: principal.user.name,
                    },
                    where: Condition.all(),
                },
                {
                    type: {
                        packageId: account.policy.definition.packageId,
                        type: account.policy.definition.name,
                    },
                    where: Condition.all(),
                    within: [
                        {
                            packageId: principal.user.definition.packageId,
                            type: principal.user.name,
                        },
                    ],
                },
            ],
        },
        { signal: controller.signal },
    );
    const page = (await pages[Symbol.asyncIterator]().next()).value as QueryPage;
    controller.abort();

    return page.changes
        .filter((change) => change.table !== "destack__sync__replica")
        .map((change) => [
            change.operation,
            change.row.id,
            change.table === "destack__account__user" ? change.row.name : change.row.handle,
            change.concealed,
        ]);
}
