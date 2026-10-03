import { TEST_DIALECTS } from "@destack/db/test";
import { Scope } from "@destack/sync";
import { copyScope } from "@destack/access/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { asc, eq } from "@destack/db";
import { Stack } from "@destack/object/server";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { defineSetting } from "../src/declare/index.ts";
import { setting } from "../src/object/index.ts";
import { servedObjects } from "../src/server/index.ts";
import { editor, notes } from "./fixture/settings/editor.ts";
import { Storage } from "./fixture/storage.ts";

/** A space-wide template the space's stack selects. */
const template = defineSetting(
    { ...editor.definition, name: "template", scope: "space", overrides: ["installation"] },
    { package: notes },
);

/** The stack applying the values. */
const manager = {
    installationId: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000001"),
    packageId: schema.identifier("package").parse("package-01996ab0-0000-7000-8000-000000000002"),
};

/** The space receiving the values. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

test.each(TEST_DIALECTS)(
    "set, recommend and require declared values in the space and refuse undeclared or misplaced ones on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect, [editor, template]);
        onTestFinished(() => storage.close());
        await copyScope(storage.database, space.reference(Scope.universe.id, spaceId));
        const { release } = storage;
        const objects = [servedObjects(release).setting];
        const apply = (settings: Readonly<Record<string, unknown>>) =>
            Stack.apply({
                database: storage.database,
                objects,
                release: (packageId, installationId) => release(spaceId, packageId, installationId),
                manager,
                scope: spaceId,
                document: { settings },
            });

        // set the space template and recommend the editor mode to the space's users
        await apply({
            template: { setting: template.reference, value: "vim", mode: "set" },
            editor: { setting: editor.reference, value: "vim", mode: "recommend" },
        });
        const rows = await storage.database
            .select()
            .from(setting.table)
            .orderBy(asc(setting.table.name));
        const written = {
            createdBy: null,
            updatedBy: null,
            scope: spaceId,
            revision: 1,
            tags: {},
            managerInstallationId: manager.installationId,
            managerPackageId: manager.packageId,
            detachedAt: null,
            packageId: notes.id,
            package: null,
            space: null,
            installation: null,
            deviceId: null,
            value: "vim",
            release: "2026.9.0",
        };
        const [editorRow, templateRow] = rows;
        if (editorRow === undefined || templateRow === undefined) {
            throw new TypeError("the stack placed fewer than two values");
        }
        expect(rows).toEqual([
            {
                ...written,
                id: editorRow.id,
                createdAt: editorRow.createdAt,
                updatedAt: editorRow.updatedAt,
                managerName: "editor",
                name: "editor.mode",
                mode: "recommend",
            },
            {
                ...written,
                id: templateRow.id,
                createdAt: templateRow.createdAt,
                updatedAt: templateRow.updatedAt,
                managerName: "template",
                name: "template",
                mode: "set",
            },
        ]);

        // refuse values outside the setting's schema, undeclared settings,
        //  and a user's value set in the space, which may only recommend or require it
        const unknown = { ...editor.reference, name: "editor.theme" };
        expect([
            await refusal(
                apply({
                    template: { setting: template.reference, value: "emacs", mode: "set" },
                }),
            ),
            await refusal(apply({ theme: { setting: unknown, value: "vim", mode: "recommend" } })),
            await refusal(
                apply({ editor: { setting: editor.reference, value: "vim", mode: "set" } }),
            ),
        ]).toEqual([
            ["BAD_REQUEST", "setting value does not match its declaration"],
            ["NOT_FOUND", `setting ${notes.id}/editor.theme is not declared`],
            ["BAD_REQUEST", "setting value outside its declared scope must recommend or require"],
        ]);

        // turn the recommendation into a requirement of the space's users
        await apply({
            template: { setting: template.reference, value: "vim", mode: "set" },
            editor: { setting: editor.reference, value: "vim", mode: "require" },
        });
        const [requirement] = await storage.database
            .select({ mode: setting.table.mode, revision: setting.table.revision })
            .from(setting.table)
            .where(eq(setting.table.name, "editor.mode"));
        expect(requirement).toEqual({ mode: "require", revision: 2 });
    },
);
