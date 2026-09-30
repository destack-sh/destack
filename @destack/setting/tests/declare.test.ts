import { TEST_DIALECTS } from "@destack/db/test";
import { Scope } from "@destack/sync";
import { copyScope } from "@destack/access/test";
import { expect, onTestFinished, test } from "@destack/test";
import { asc, eq } from "@destack/db";
import { Stack } from "@destack/object/server";
import { identifier } from "@destack/schema";
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
    installationId: identifier("installation").parse(
        "installation-01996ab0-0000-7000-8000-000000000001",
    ),
    packageId: identifier("package").parse("package-01996ab0-0000-7000-8000-000000000002"),
};

/** The space receiving the values. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

test.each(TEST_DIALECTS)(
    "set, recommend and require declared values in the space and refuse undeclared or misplaced ones on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect, [editor, template]);
        onTestFinished(() => storage.close());
        await copyScope(storage.database, space.reference(Scope.universe.id, spaceId));
        const { release } = storage.options;
        const objects = [servedObjects(release).setting];
        const apply = (settings: Readonly<Record<string, unknown>>) =>
            Stack.apply({
                database: storage.database,
                objects,
                release,
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
            device: null,
            value: "vim",
            release: "2026.9.0",
        };
        expect(rows).toEqual([
            {
                ...written,
                id: rows[0]!.id,
                createdAt: rows[0]!.createdAt,
                updatedAt: rows[0]!.updatedAt,
                managerName: "editor",
                name: "editor.mode",
                mode: "recommend",
            },
            {
                ...written,
                id: rows[1]!.id,
                createdAt: rows[1]!.createdAt,
                updatedAt: rows[1]!.updatedAt,
                managerName: "template",
                name: "template",
                mode: "set",
            },
        ]);

        // refuse values outside the setting's schema, undeclared settings,
        //  and a user's value set in the space, which may only recommend or require it
        const unknown = { ...editor.reference, name: "editor.theme" };
        const refusal = (settings: Readonly<Record<string, unknown>>) =>
            apply(settings).then(
                () => "accepted",
                (error: { code: string; message: string }) => ({
                    code: error.code,
                    message: error.message,
                }),
            );
        expect([
            await refusal({
                template: { setting: template.reference, value: "emacs", mode: "set" },
            }),
            await refusal({ theme: { setting: unknown, value: "vim", mode: "recommend" } }),
            await refusal({ editor: { setting: editor.reference, value: "vim", mode: "set" } }),
        ]).toEqual([
            { code: "BAD_REQUEST", message: "setting value does not match its declaration" },
            { code: "NOT_FOUND", message: `setting ${notes.id}/editor.theme is not declared` },
            {
                code: "BAD_REQUEST",
                message: "setting value outside its declared scope must recommend or require",
            },
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
