import { FOLLOWER_REVISION } from "@destack/sync/test";
import { Snapshot } from "@destack/db";
import { message } from "@destack/message";
import { journal, QueriesParameters, Shape } from "@destack/object";
import objectPackage from "@destack/object/package";
import { Account, account, space } from "@destack/account/object";
import { subjectContext } from "@destack/object/test";
import { present, schema } from "@destack/schema";
import { type Page, RequestId, Scope, Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { activity, activityShape, notification } from "../src/index.ts";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

/** How long a relayed page may take on the world's clock before the read gives up. */
const RELAY_TIMEOUT = 5000;

test("replicate a person's activities from a space as the person, and none of them to another reader of their source", async () => {
    // mention alice on a document bob views
    await using fixture = await TestNotification.open();
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "viewer",
            subject: fixture.subject("bob"),
        }),
    );
    await fixture.call(
        "bob",
        document.call("remark", {
            ...fixture.scope,
            id: plan.id,
            text: "@alice, a question",
            mentions: [fixture.subject("alice")],
        }),
    );

    // read the first page of alice's activities as alice, then as bob
    const first = async (person: Subject) => {
        const subscription = Shape.subscription(
            fixture.scope.spaceId,
            [{ shape: activityShape, keys: [{ recipient: fixture.subject("alice").id }] }],
            "space-019f5530-8000-7000-8000-0000000000aa",
            person,
        );
        const { environment } = fixture;
        const signal = environment.clock.timeout(RELAY_TIMEOUT);
        const pages = fixture.served.store.source.relayed(
            { ...subscription, revision: FOLLOWER_REVISION },
            subjectContext(person, fixture.scope.spaceId, { signal, environment }),
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
    expect([await first(fixture.subject("alice")), await first(fixture.subject("bob"))]).toEqual([
        [["insert", fixture.subject("alice").id]],
        [],
    ]);
});

test("seal an emailed notification's content to its home's message key, so no journal entry, notification or message change contains its text", async () => {
    // give alice an email address, and let bob mention her on a document she views
    await using fixture = await TestNotification.open();
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "editor",
            subject: fixture.subject("bob"),
        }),
    );
    const text = "@alice, the sealed question";
    await fixture.call(
        "bob",
        document.call("remark", {
            ...fixture.scope,
            id: plan.id,
            text,
            mentions: [fixture.subject("alice")],
        }),
    );

    // email it once the delay passes
    await fixture.dispatch();
    await fixture.wait(16);

    // list where the text appears in alice's home beside the activity replica: the journal, the notifications' delivery state, and the messages' changes
    const { served: home } = await fixture.home("alice");
    const traces = {
        journal: await home.database.select().from(journal),
        notifications: await home.database.select().from(notification.table),
        log: (await home.database.log.read({ tables: [message.table], after: 0 })).changes,
    };
    const leaking = Object.entries(traces).flatMap(([name, rows]) =>
        JSON.stringify(rows).includes(text) ? [name] : [],
    );
    expect({
        leaking,
        sent: fixture.mails.sent.map((mail) => [mail.to, schema.string().parse(mail.text)]),
    }).toEqual({ leaking: [], sent: [["alice@example.test", `⁨${text}⁩`]] });
});

test("follow the spaces of an account once the person owns it, and their activities", async () => {
    // list alice's home's follows of the fixture account's spaces and their activities
    await using fixture = await TestNotification.open();
    const { served: home } = await fixture.home("alice");
    const follows = async () => {
        await fixture.dispatch();
        const subscriptions = await present(
            home.store.subscriber,
            "the home's subscriber",
        ).subscriptions(Snapshot.live(home.store.database));

        const listed = subscriptions
            .map((entry) => entry.subscription)
            .filter((subscription) => subscription.name.startsWith("follow:"))
            .flatMap((subscription) =>
                Object.entries(QueriesParameters.parse(subscription.parameters).queries ?? {}).map(
                    ([key, query]) => [key.split(":")[0], subscription.scope, query],
                ),
            )
            .filter(([shape]) => shape === "account-space" || shape === "activity");

        return sorted(listed);
    };
    const guest = await follows();

    // let the account's owner make alice an owner of the account too, its home served beside the space
    await fixture.serve(Account.home(fixture.space.account));
    await fixture.space.owner.client(objectPackage).relationship.create({
        scope: fixture.space.owner.id,
        requestId: RequestId.create(fixture.environment),
        object: account.reference(fixture.space.owner.id, fixture.space.account),
        relation: "owner",
        subject: fixture.subject("alice"),
    });
    const owner = await follows();

    // follow the space and her personal account's spaces as its guest, then the account's spaces and their activities as their owner
    const recipientId = fixture.subject("alice").id;
    const activities = (spaceId: string) => [
        "activity",
        spaceId,
        { object: "activity", package: activity.package.id, where: { recipientId } },
    ];
    const personal = Account.personal(recipientId);
    expect({ guest, owner }).toEqual({
        guest: sorted([accountSpaces(personal), activities(fixture.scope.spaceId)]),
        owner: sorted([
            accountSpaces(personal),
            accountSpaces(fixture.space.account),
            activities(Account.home(fixture.space.account)),
            activities(fixture.scope.spaceId),
        ]),
    });
});

/** Follow the spaces of an account, as the account-space query lists them. */
function accountSpaces(accountId: string): unknown[] {
    return [
        "account-space",
        Scope.universe.id,
        {
            object: "space",
            package: space.package.id,
            scopes: "every",
            where: { scope: accountId },
        },
    ];
}

/** Sort follows by their JSON, as the home lists them in no fixed order. */
function sorted(follows: readonly unknown[]): unknown[] {
    return follows.toSorted((left, right) =>
        JSON.stringify(left).localeCompare(JSON.stringify(right)),
    );
}
