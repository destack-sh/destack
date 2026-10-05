import { plural, t } from "@destack/locale";
import { expect, test } from "@destack/test";
import { found, schema } from "@destack/schema";
import { defineNotification } from "../src/declare/index.ts";
import { PackageFile } from "@destack/package/file";
import { BuildReader } from "@destack/package/manifest";
import { graph } from "@destack/package";
import { notification } from "../src/index.ts";
import {
    describeNotification,
    describeNotificationPreference,
    notificationVocabulary,
    readNotifications,
} from "../src/inspect/index.ts";
import { mention, notes } from "./fixture/document.ts";

/** A notification description read back from its JSON. */
const Described = schema.record(schema.string(), schema.json());

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
    const described = Described.parse(JSON.parse(JSON.stringify(describeNotification(mention))));
    expect(notificationVocabulary(described)).toEqual({
        mention: { payload: described["payload"] },
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

test("read the notifications a build declares from its graph, and none another package declares", async () => {
    // declare the mention, and a notification of the notes package's kind beside it
    const symbol = graph.Moniker.of({
        packageId: notes.id,
        module: "src/notification.ts",
        name: "mention",
    });
    const declaration = (packageId: typeof notes.id): graph.Declaration => ({
        moniker: graph.Moniker.parse(`${symbol}:notification`),
        symbol,
        kind: "notification",
        package: packageId,
        name: "mention",
        description: describeNotification(mention),
    });
    const module = await graph.Module.file({
        path: "src/notification.ts",
        digest: "0".repeat(64),
        imports: [],
        exports: [],
        symbols: [],
        declarations: [declaration(notification.package.id), declaration(notes.id)],
        edges: [],
    });

    // keep the module's graph file under the root, and the manifest's other files empty
    const files = new Map([[`graph/${module.digest}.json`, module.bytes]]);
    const root = new TextEncoder().encode(
        JSON.stringify({ modules: { "src/notification.ts": module.digest } }),
    );
    files.set("manifest/graph.json", root);
    const empty = await PackageFile.describe(
        "manifest/empty.json",
        "application/json",
        new Uint8Array(),
    );
    const reader = new BuildReader(
        {
            formatVersion: 1,
            package: notes,
            language: "typescript",
            lists: {
                dependencies: empty,
                files: empty,
                sourceMaps: empty,
                graph: await PackageFile.describe("manifest/graph.json", "application/json", root),
            },
            outputs: {},
        },
        async (path) => found(files, path),
    );

    // read the described mention once, leaving the notes package's kind out
    expect(await readNotifications(reader)).toEqual([describeNotification(mention)]);
});

/** Declare a review notification under a name with one action, deferred until called. */
function declare(name: string, action: string, title: string) {
    return () =>
        defineNotification(
            {
                name,
                title: "Reviews",
                description: "A review waits for you.",
                payload: schema.object({}),
                interruption: "active",
                preference: { channels: [], delivery: "immediate" },
                content: () => ({ title: "Review", body: "" }),
                summary: (count) => t`${plural(count, { one: "# review", other: "# reviews" })}`,
                actions: {
                    [action]: {
                        title,
                        effect: async () => undefined,
                    },
                },
            },
            { package: notes },
        );
}

/** Read the issues a refused declaration names, by code and path. */
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
