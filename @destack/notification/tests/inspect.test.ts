import { argument, type Message, plural, t } from "@destack/locale";
import { expect, expectTypeOf, test } from "@destack/test";
import { schema } from "@destack/schema";
import { describeSetting } from "@destack/setting/inspect";
import { defineNotification } from "../src/declare/index.ts";
import {
    describeNotification,
    notificationPreference,
    notificationVocabulary,
} from "../src/inspect/index.ts";
import type { MutationName } from "@destack/object";
import {
    type ActionDefinition,
    type NotificationDefinition,
    type PayloadField,
} from "../src/notification/index.ts";
import { document, mention, notes } from "./fixture/document.ts";

/** A notification description read back from its JSON. */
const Described = schema.jsonObject();

test("describe a declared notification for manifests with its actions, and its preference as a setting", () => {
    // describe the notification
    expect(describeNotification(mention)).toEqual({
        name: "mention",
        title: "Mentions",
        description: "Someone mentions you in a remark.",
        interruption: "active",
        preference: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        package: notes,
        payload: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "object",
            properties: { author: { type: "string" }, excerpt: { type: "string", maxLength: 280 } },
            required: ["author", "excerpt"],
            additionalProperties: false,
        },
        content: {
            title: {
                package: notes.id,
                id: "h0a6nydo5bqi",
                source: "{$author} mentioned you",
                values: {},
            },
            body: mention.definition.content.body,
        },
        summary: mention.definition.summary,
        actions: [
            {
                name: "reply",
                title: "Reply",
                text: { placeholder: "Reply", button: "Send" },
                type: document.typeReference,
                method: "reply",
            },
        ],
    });

    // fix the notification's name in stored notifications with its payload's shape
    const described = Described.parse(JSON.parse(JSON.stringify(describeNotification(mention))));
    expect(notificationVocabulary(mention)).toEqual({
        mention: { payload: described["payload"] },
    });

    // derive the preference as the user setting notification.mention of the declaring package
    expect(describeSetting(notificationPreference(mention))).toEqual({
        name: "notification.mention",
        title: "Mentions",
        description: "Someone mentions you in a remark.",
        apply: "immediate",
        scope: "user",
        overrides: ["space", "installation", "device"],
        package: notes,
        default: { channels: ["desktop", "push", "email"], delivery: "immediate" },
        schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "object",
            properties: {
                channels: {
                    type: "array",
                    items: { type: "string", enum: ["desktop", "push", "email"] },
                },
                delivery: { type: "string", enum: ["immediate", "summary"] },
            },
            required: ["channels", "delivery"],
            additionalProperties: false,
        },
    });
});

test("refuse declarations whose names cannot identify a setting, and unlabeled actions", () => {
    // accept camel case names, and refuse kebab case names and empty action titles
    expect([
        declare("reviewRequested", "approve", "Approve")().preference.name,
        issues(declare("review-requested", "approve", "Approve")),
        issues(declare("review", "approve-all", "Approve")),
        issues(declare("review", "approve", "")),
    ]).toEqual([
        "notification.reviewRequested",
        [["invalid_format", ["name"]]],
        [["invalid_format", ["name"]]],
        [["too_small", ["title"]]],
    ]);
});

test("type a declaration's content by its payload's fields, its summary by the count and its actions by mutating methods", () => {
    // take the payload's fields as the content's arguments, and only the count as the summary's
    expectTypeOf<PayloadField<typeof mention.definition.payload>>().toEqualTypeOf<
        "author" | "excerpt"
    >();
    expectTypeOf<NotificationDefinition["summary"]>().toEqualTypeOf<Message<"count">>();

    // take a mutating method of the type, and a string field as the target
    type Approval = ActionDefinition<{ id: string; count: number }, typeof document>;
    expectTypeOf<Approval["method"]>().toEqualTypeOf<MutationName<typeof document>>();
    expectTypeOf<Approval["target"]>().toEqualTypeOf<"id" | undefined>();
});

/** Declare a review notification under a name with one action, deferred until called. */
function declare(name: string, action: string, title: string) {
    return () =>
        defineNotification(
            {
                name,
                title: "Reviews",
                description: "A review waits for you.",
                payload: schema.object({ id: schema.string() }),
                interruption: "active",
                preference: { channels: [], delivery: "immediate" },
                content: { title: t`Review`, body: t`${argument("id")}` },
                summary: t`${plural(argument("count"), { one: "# review", other: "# reviews" })}`,
                actions: [{ name: action, title, type: document, method: "approve", target: "id" }],
            },
            { package: notes },
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
