import { ObjectReference } from "@destack/sync";
import { expect, refusal, single, test } from "@destack/test";
import { asc, eq } from "@destack/db";
import { Stack } from "@destack/object/server";
import { nth, schema, type JsonObject } from "@destack/schema";

import { defineSetting } from "../src/declare/index.ts";
import { setting } from "../src/object/index.ts";
import type { SettingDeclared } from "../src/setting/index.ts";
import { editor, notes, release } from "./fixture/setting/index.ts";
import { openSettings } from "./fixture/store.ts";

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

test("set, recommend and require declared values in the space and refuse undeclared or misplaced ones", async () => {
    // serve the space's settings, checked against the build declaring them
    await using served = await openSettings({ settings: [editor, template] });
    const spaceId = served.scope;
    const objects = served.store.objects.filter((type) => type.same(setting));
    const reader = release([editor, template]);
    let applied: JsonObject = {};
    const apply = async (settings: JsonObject) => {
        // apply the settings against the ones applied last
        const document = { settings };
        const plan = await new Stack({
            database: served.database,
            isHome: false,
            objects,
            store: served.store,
            as: served.store.principal,
            release: async () => await reader,
            manager,
            scope: spaceId,
            document,
            applied,
        }).apply(served.environment.clock.now());
        applied = document;

        return plan;
    };

    // set the space template and recommend the editor mode to the space's users
    await apply({
        template: { ...template.reference, value: "vim", mode: "set" },
        editor: { ...editor.reference, value: "vim", mode: "recommend" },
    });
    const rows = await served.database
        .select()
        .from(setting.table)
        .orderBy(asc(setting.table.name));
    const written = {
        ...ObjectReference.columns("createdBy", served.store.principal),
        ...ObjectReference.columns("updatedBy", served.store.principal),
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
        value: "vim",
        release: "2026.9.0",
    };
    const editorRow = nth(rows, 0);
    const templateRow = nth(rows, 1);
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
                template: { ...template.reference, value: "emacs", mode: "set" },
            }),
        ),
        await refusal(apply({ theme: { ...unknown, value: "vim", mode: "recommend" } })),
        await refusal(apply({ editor: { ...editor.reference, value: "vim", mode: "set" } })),
    ]).toEqual([
        ["BAD_REQUEST", "VALUE_INVALID", "setting value does not match its declaration"],
        ["NOT_FOUND", "SETTING_UNDECLARED", `setting ${notes.id}/editor.theme is not declared`],
        [
            "BAD_REQUEST",
            "PLACEMENT_INVALID",
            "setting value outside its declared scope must be a recommendation or a requirement",
        ],
    ]);

    // turn the recommendation into a requirement of the space's users
    await apply({
        template: { ...template.reference, value: "vim", mode: "set" },
        editor: { ...editor.reference, value: "vim", mode: "require" },
    });
    const requirement = single(
        await served.database
            .select()
            .from(setting.table)
            .where(eq(setting.table.name, "editor.mode")),
    );
    expect(requirement).toEqual({
        ...written,
        id: editorRow.id,
        createdAt: editorRow.createdAt,
        updatedAt: requirement.updatedAt,
        managerName: "editor",
        name: "editor.mode",
        mode: "require",
        revision: 2,
    });
});

test("stamp a declared value with the release of the build declaring its setting, then check it at that release", async () => {
    // apply a recommendation of a setting its build declares, recording each build the store opens
    const opened: SettingDeclared[] = [];
    await using served = await openSettings({ settings: [editor], opened });
    const reader = release([editor]);
    await new Stack({
        database: served.database,
        isHome: false,
        objects: served.store.objects.filter((type) => type.same(setting)),
        store: served.store,
        as: served.store.principal,
        release: async () => await reader,
        manager,
        scope: served.scope,
        document: {
            settings: { editor: { ...editor.reference, value: "vim", mode: "recommend" } },
        },
        applied: {},
    }).apply(served.environment.clock.now());

    // stamp the value with the installed build's release, then check the write against that release
    const placed = single(await served.database.select().from(setting.table));
    const declared = { ...editor.reference, scope: served.scope };
    expect([placed.release, opened]).toEqual([
        editor.package.version,
        [declared, { ...declared, release: editor.package.version }],
    ]);
});
