import { TEST_DIALECTS } from "@destack/db/test";
import { subscribeBrowser } from "@destack/message/test";
import { aligned, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Bob's old phone, whose push service forgot it. */
const OLD_PHONE = schema
    .identifier("push-endpoint")
    .parse("push-endpoint-019f5530-8000-7000-8000-0000000000b1");

/** Bob's new phone. */
const NEW_PHONE = schema
    .identifier("push-endpoint")
    .parse("push-endpoint-019f5530-8000-7000-8000-0000000000b2");

test.for(TEST_DIALECTS)(
    "push a mention to each of a recipient's phones as a message, and forget the phone its push service forgot through the push endpoint kind, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // give bob two phones, the old one forgotten by its push service
        const browser = await subscribeBrowser();
        await fixture.subscribe("bob", {
            id: OLD_PHONE,
            url: "https://push.example/bob-old",
            keys: browser.keys,
        });
        await fixture.subscribe("bob", {
            id: NEW_PHONE,
            url: "https://push.example/bob-new",
            keys: browser.keys,
        });
        fixture.pushes.gone.add("https://push.example/bob-old");

        // mention bob on a document he views
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        await fixture.call(document, "grant", {
            id: plan.id,
            relation: "viewer",
            subject: actors.bob,
        });
        const box = fixture.inbox("bob");
        await fixture.call(document, "remark", {
            id: plan.id,
            text: "@bob, a question",
            mentions: [actors.bob],
        });
        await box.until((state) => state.unread === 1);
        await fixture.dispatch();

        // send one push to each phone, skip the old phone's delivery as gone and forget the old phone
        const mentioned = aligned(await fixture.notifications(), 0);
        expect({
            pushed: fixture.pushes.requests.map((request) => request.url).toSorted(),
            deliveries: (await fixture.deliveries(mentioned.id)).map((row) => [
                row.channel,
                row.endpoint,
                row.state,
                row.reason,
            ]),
            forgotten: fixture.forgotten,
        }).toEqual({
            pushed: ["https://push.example/bob-new", "https://push.example/bob-old"],
            deliveries: [
                ["email", null, "skipped", "unaddressed"],
                ["push", OLD_PHONE, "skipped", "gone"],
                ["push", NEW_PHONE, "sent", null],
            ],
            forgotten: [OLD_PHONE],
        });
    },
);
