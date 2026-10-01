import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { defineNotification } from "../src/declare/index.ts";
import { describeFile } from "@destack/package/file";
import { BuildReader, type PackageManifest } from "@destack/package/manifest";
import { notification } from "../src/index.ts";
import {
    describeNotification,
    describeNotificationPreference,
    notificationVocabulary,
    readNotifications,
} from "../src/inspect/index.ts";
import { mention, notes } from "./fixture/document.ts";

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
        actions: { reply: { title: "Reply", text: { placeholder: "Reply", button: "Send" } } },
    });

    // fix the notification's name in stored notifications with its payload's shape
    const described = JSON.parse(JSON.stringify(describeNotification(mention)));
    expect(notificationVocabulary(described)).toEqual({
        mention: { payload: described.payload },
    });

    // describe the preference as the user setting notification.mention of the declaring package
    expect(describeNotificationPreference(mention)).toEqual({
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
    const declare = (name: string, action: string, title: string) => () =>
        defineNotification(
            {
                name,
                title: "Reviews",
                description: "A review waits for you.",
                payload: schema.object({}),
                interruption: "active",
                preference: { channels: [], delivery: "immediate" },
                content: () => ({ title: "Review", body: "" }),
                summary: (count) => `${count} reviews`,
                actions: {
                    [action]: {
                        title,
                        effect: async () => undefined,
                    },
                },
            },
            { package: notes },
        );

    // read the issues a refused declaration names, by code and path
    const issues = (build: () => unknown) => {
        try {
            build();
        } catch (error) {
            return (error as schema.Error).issues.map((issue) => [issue.code, issue.path]);
        }

        return [];
    };

    // accept camel case names, and refuse kebab case names and empty action titles
    expect([
        declare("reviewRequested", "approve", "Approve")().preference.name,
        issues(declare("review-requested", "approve", "Approve")),
        issues(declare("review", "approve-all", "Approve")),
        issues(declare("review", "approve", "")),
    ]).toEqual([
        "notification.reviewRequested",
        [["invalid_format", ["name"]]],
        [["invalid_format", []]],
        [["too_small", ["title"]]],
    ]);
});

test("read the notifications a build declares from this package's description collection, and none from a build declaring none", async () => {
    // keep a manifest with the mention in its notification collection and one without
    const declaration = {
        name: "mention",
        kind: "notification",
        package: notification.package,
        constructor: {
            package: notification.package,
            symbol: { module: "src/declare/notification.ts", name: "defineNotification" },
        },
        symbol: { package: notes, symbol: { module: "src/notification.ts", name: "mention" } },
        source: { file: "src/notification.ts", line: 0, column: 0 },
        description: describeNotification(mention),
    };
    const bytes = new TextEncoder().encode(JSON.stringify([declaration]));
    const file = await describeFile("manifest/notification.json", "application/json", bytes);
    const reader = (descriptions: PackageManifest["descriptions"]) =>
        // only the descriptions matter to reading notifications
        new BuildReader({ descriptions } as PackageManifest, async () => bytes);

    // read the described mention, and nothing where no collection names this package
    expect([
        await readNotifications(reader({ notification: { package: notification.package, file } })),
        await readNotifications(reader({ notes: { package: notes, file } })),
    ]).toEqual([[describeNotification(mention)], []]);
});
