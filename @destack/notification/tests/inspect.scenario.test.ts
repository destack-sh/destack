import { argument, type Message, plural, t } from "@destack/locale";
import { expect, expectTypeOf, test } from "@destack/test";
import { schema } from "@destack/schema";
import { defineNotification } from "../src/declare/index.ts";
import { describeNotification, notificationVocabulary } from "../src/inspect/index.ts";
import { type Action, defineAction } from "@destack/object";
import {
    type NotificationActionDefinition,
    type NotificationDefinition,
    type PayloadField,
} from "../src/notification/index.ts";
import { Preference } from "../src/preference/index.ts";
import { approve, document, mention, NOTES, reply } from "../src/test/notes.ts";

/** A notification description read back from its JSON. */
const Described = schema.jsonObject();

test("describe a declared notification for manifests with its actions, and key its preference", () => {
    // describe the notification
    expect(describeNotification(mention)).toEqual({
        name: "mention",
        title: mention.definition.title,
        description: mention.definition.description,
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        package: NOTES,
        payload: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "object",
            properties: { author: { type: "string" }, excerpt: { type: "string", maxLength: 280 } },
            required: ["author", "excerpt"],
            additionalProperties: false,
        },
        content: {
            title: {
                package: NOTES.id,
                id: "h0a6nydo5bqi",
                source: "{$author} mentioned you",
                values: {},
            },
            body: mention.definition.content.body,
        },
        summary: mention.definition.summary,
        actions: [
            {
                name: "document.reply",
                title: reply.title,
                text: { placeholder: "Reply", button: "Send" },
                call: {
                    ...document.typeReference,
                    method: "reply",
                    input: { text: { placeholder: "text" } },
                    target: "documents",
                    inputs: [
                        {
                            kind: "object",
                            name: "documents",
                            presence: "required",
                            source: "selected",
                            type: document.typeReference,
                        },
                        { kind: "text", name: "text", presence: "required" },
                    ],
                },
            },
        ],
    });

    // fix the notification's name in stored notifications with its payload's shape
    const described = Described.parse(JSON.parse(JSON.stringify(describeNotification(mention))));
    expect(notificationVocabulary(mention)).toEqual({
        mention: { payload: described["payload"] },
    });

    // key its preference by its package and name in a person's preferences
    expect(Preference.key(mention.reference)).toEqual(`${NOTES.id}/mention`);
});

test("refuse declarations whose names cannot key a preference, and actions calling no method", () => {
    // accept camel case names, and refuse kebab case names and an action the open view handles
    const zoom = defineAction({ name: "zoom", title: t`Zoom` }, { package: NOTES });
    expect([
        declare("reviewRequested", approve)().name,
        issues(declare("review-requested", approve)),
    ]).toEqual(["reviewRequested", [["invalid_format", ["name"]]]]);
    expect(declare("review", zoom)).toThrow(
        new TypeError("action 'zoom' of 'review' calls no method"),
    );
});

test("type a declaration's content by its payload's fields, its summary by the count and its actions' targets by string fields", () => {
    // take the payload's fields as the content's arguments, and only the count as the summary's
    expectTypeOf<PayloadField<typeof mention.definition.payload>>().toEqualTypeOf<
        "author" | "excerpt"
    >();
    expectTypeOf<NotificationDefinition["summary"]>().toEqualTypeOf<Message<"count">>();

    // take a string field as an action's target
    type Approval = NotificationActionDefinition<{ id: string; count: number }>;
    expectTypeOf<Approval["target"]>().toEqualTypeOf<"id" | undefined>();
});

/** Declare a review notification under a name with one action, deferred until called. */
function declare(name: string, action: Action) {
    return () =>
        defineNotification(
            {
                name,
                title: t`Reviews`,
                description: t`A review waits for you.`,
                payload: schema.object({ id: schema.string() }),
                interruption: "active",
                preference: { channels: [], delivery: "immediate" },
                content: { title: t`Review`, body: t`${argument("id")}` },
                summary: t`${plural(argument("count"), { one: "# review", other: "# reviews" })}`,
                actions: [{ action, target: "id" }],
            },
            { package: NOTES },
        );
}

/** Read the issues refusing a declaration, by code and path. */
function issues(build: () => unknown) {
    try {
        build();
    } catch (error) {
        // read a schema refusal, and rethrow anything else
        if (error instanceof schema.Error) {
            return error.issues.map((issue) => [issue.code, issue.path]);
        }
        throw error;
    }

    return [];
}
