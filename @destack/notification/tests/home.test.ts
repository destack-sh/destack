import { STACKS } from "@destack/account/test";
import { Table } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { message } from "@destack/message";
import { journal } from "@destack/object";
import { subjectContext } from "@destack/runtime/test";
import { Identifier, present, schema } from "@destack/schema";
import { type Page, Scope, Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { activityShape, deliver } from "../src/index.ts";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "replicate a person's activities from a space as the person, and none of them to another reader of their source, on %s",
    async (dialect) => {
        // mention alice on a document bob views
        const fixture = await serveSpace(dialect);
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "viewer",
                subject: actors.bob,
            }),
        );
        fixture.as("bob");
        await fixture.call(
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text: "@alice, a question",
                mentions: [actors.alice],
            }),
        );

        // read the first page of alice's activities as alice, then as bob
        const first = async (person: Subject) => {
            const subscription = activityShape.subscription(
                { scope: fixture.spaceId, recipient: actors.alice.id },
                "space-019f5530-8000-7000-8000-0000000000aa",
                person,
            );
            const signal = AbortSignal.timeout(5000);
            const pages = fixture.store.source.relayed(
                subscription,
                subjectContext(person, fixture.spaceId, { signal }),
            );
            const read: Page[] = [];
            for await (const page of pages) {
                read.push(page);
                break;
            }

            return read.flatMap((page) =>
                page.changes.map((change) => [change.operation, change.row["recipientId"]]),
            );
        };
        expect([await first(actors.alice), await first(actors.bob)]).toEqual([
            [["insert", actors.alice.id]],
            [],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "seal an emailed notification's content to its home's message key, so no journal entry, delivery run or step, or message change contains its text, on %s",
    async (dialect) => {
        // give alice an email address, and let bob mention her on a document she views
        const fixture = await serveSpace(dialect);
        await fixture.contact("alice", { email: "alice@example.com" });
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "editor",
                subject: actors.bob,
            }),
        );
        const text = "@alice, the sealed question";
        fixture.as("bob");
        await fixture.call(
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text,
                mentions: [actors.alice],
            }),
        );

        // email it once the delay passes
        await fixture.dispatch();
        await fixture.wait(16);
        await fixture.dispatch();

        // list where the text appears in alice's home beside the activity replica: the journal, the delivery's runs and steps, and the messages' changes
        const { test: home } = await fixture.home("alice");
        const traces = {
            journal: await home.database.select().from(journal),
            runs: await home.database.select().from(deliver.run.table),
            steps: await home.database.select().from(deliver.step.table),
            log: (await home.database.log.read({ tables: [message.table], after: 0 })).changes,
        };
        const leaking = Object.entries(traces).flatMap(([name, rows]) =>
            JSON.stringify(rows).includes(text) ? [name] : [],
        );
        expect({
            leaking,
            sent: fixture.mails.sent.map((mail) => [mail.to, schema.string().parse(mail.text)]),
        }).toEqual({ leaking: [], sent: [["alice@example.com", `⁨${text}⁩`]] });
    },
);

test.for(TEST_DIALECTS)(
    "follow the activity of a space the person reaches through owning its account, on %s",
    async (dialect) => {
        // keep alice's ownership of another account and its board space, as the home's replicas bring them
        const fixture = await serveSpace(dialect);
        const home = await fixture.home("alice");
        const accountId = Identifier.create("account");
        const boardId = Identifier.create("space");
        const board = {
            id: boardId,
            scope: accountId,
            name: "board",
            stack: STACKS.space,
            createdAt: 1,
            updatedAt: 1,
        };
        await home.test.database.insert(space.table).values(Table.complete(space.table, board));

        // keep a space of an account she has no relationship on, which she does not follow
        const elsewhere = {
            ...board,
            id: Identifier.create("space"),
            scope: Identifier.create("account"),
        };
        await home.test.database.insert(space.table).values(Table.complete(space.table, elsewhere));
        const owned = account.reference(Scope.universe.id, accountId);
        await new AccessFixture(home.test.database).replicateRelationship(
            Scope.universe.id,
            owned,
            "owner",
            actors.alice,
            1,
        );

        // list the home's follows of the account's spaces and of activities
        const recipientId = actors.alice.id;
        const subscriptions =
            (await present(home.store.subscriber, "the home's subscriber").subscriptions?.()) ?? [];
        const follows = subscriptions
            .map((entry) => entry.subscription)
            .map((subscription) => [
                subscription.name.split(":")[0],
                subscription.scope,
                subscription.parameters,
            ])
            .filter(([shape]) => shape === "account-space" || shape === "activity");

        expect(follows).toEqual([
            [
                "account-space",
                Scope.universe.id,
                {
                    queries: {
                        "0:space": {
                            object: "space",
                            scopes: "every",
                            where: { scope: accountId },
                        },
                    },
                },
            ],
            [
                "activity",
                boardId,
                { queries: { "0:activity": { object: "activity", where: { recipientId } } } },
            ],
        ]);
    },
);
