import { TEST_DIALECTS } from "@destack/db/test";
import { subscribeBrowser } from "@destack/message/test";
import { nth, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { document } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

/** Bob's old phone, whose push service forgot it. */
const OLD_PHONE = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000b1");

/** Bob's new phone. */
const NEW_PHONE = schema.identifier("client").parse("client-019f5530-8000-7000-8000-0000000000b2");

test.for(TEST_DIALECTS)(
    "push a mention to each of a recipient's phones, and forget the subscription its push service forgot through the account package, on %s",
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
        await fixture.call(
            document.call("remark", {
                ...fixture.scope,
                id: plan.id,
                text: "@bob, a question",
                mentions: [actors.bob],
            }),
        );
        await fixture.dispatch();

        // send one push to each phone, failing the old phone's as gone
        const mentioned = nth((await fixture.inbox("bob")).entries, 0);
        const messages = await fixture.messages("bob", mentioned.notification.id);
        expect({
            pushed: fixture.pushes.requests.map((request) => request.url).toSorted(),
            messages: messages
                .map((row) => [row.to.channel === "push" ? row.to.url : "", row.status, row.reason])
                .toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
        }).toEqual({
            pushed: ["https://push.example/bob-new", "https://push.example/bob-old"],
            messages: [
                ["https://push.example/bob-new", "sent", null],
                ["https://push.example/bob-old", "failed", "gone"],
            ],
        });
    },
);
