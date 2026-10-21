import { asc, Snapshot } from "@destack/db";
import type { JsonObject } from "@destack/schema";
import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { ObjectReference, Scope } from "@destack/sync";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { setting, SettingValue } from "../src/object/index.ts";
import { alice, installation, named, selection, space } from "./fixture/value.ts";
import { editor, history, lineNumbers } from "./fixture/setting/index.ts";
import { Storage } from "./fixture/storage.ts";

test.each(TEST_DIALECTS)(
    "write declared values at their placements and refuse invalid, undeclared and misplaced ones on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());
        const call = async (name: "create" | "update", input: JsonObject) =>
            await storage.send(setting, name, alice, input);
        const create = (input: JsonObject) =>
            call("create", {
                ...named(editor),
                mode: "set",
                value: "vim",
                release: "2026.9.0",
                ...input,
            });

        // write a personal value and an override in Bob's installation, and change it
        const written = {
            scope: alice,
            ...named(editor),
            mode: "set",
            release: "2026.9.0",
        } as const;
        const base = await storage.call(setting.call("create", { ...written, value: "vim" }));
        const override = await storage.call(
            setting.call("create", {
                ...written,
                installation,
                value: "standard",
            }),
        );
        const changed = await storage.call(
            setting.call("update", {
                scope: alice,
                id: override.id,
                value: "vim",
            }),
        );
        const rows = await storage.database
            .select()
            .from(setting.table)
            .orderBy(asc(setting.table.revision), asc(setting.table.installation));
        const author = principal.user.reference(Scope.universe.id, alice);
        const stored = {
            ...ObjectReference.columns("createdBy", author),
            ...ObjectReference.columns("updatedBy", author),
            scope: alice,
            ...named(editor),
            package: null,
            space: null,
            mode: "set",
            revision: 1,
            tags: {},
            managerInstallationId: null,
            managerPackageId: null,
            managerName: null,
            detachedAt: null,
            release: "2026.9.0",
        };
        expect(rows).toEqual([
            {
                ...stored,
                id: base.id,
                createdAt: base.createdAt,
                updatedAt: base.updatedAt,
                installation: null,
                value: "vim",
            },
            {
                ...stored,
                id: override.id,
                createdAt: override.createdAt,
                updatedAt: changed.updatedAt,
                revision: 2,
                installation: selection.installation,
                value: "vim",
            },
        ]);

        // refuse a second value at the same placement, values the setting rejects on creation
        //  and update, a value of a later release, an undeclared setting, and misplaced values
        const mismatch = ["INVALID_VALUE", "setting value does not match its declaration"];
        expect([
            await refusal(create({})),
            await refusal(create({ value: "emacs" })),
            await refusal(call("update", { id: base.id, value: "emacs" })),
            await refusal(create({ release: "2026.10.0" })),
            await refusal(create({ name: "editor.theme" })),
            await refusal(create({ mode: "recommend" })),
            await refusal(create({ ...named(lineNumbers), space, value: false })),
        ]).toEqual([
            [
                "CONFLICT",
                "another setting has the same packageId, name, scope, package, space, and installation",
            ],
            mismatch,
            mismatch,
            ["INVALID_VALUE", "setting value is at release 2026.10.0, its declaration at 2026.9.0"],
            ["UNDECLARED", `setting ${editor.reference.packageId}/editor.theme is not declared`],
            ["INVALID_PLACEMENT", "setting value in its declared scope must be set"],
            ["INVALID_PLACEMENT", "setting value uses an unsupported setting override"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "resolve a setting from the values placed along a scope's chain, and its default without one, on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());

        // set a personal editor mode and leave line numbers unset
        await storage.call(
            setting.call("create", {
                scope: alice,
                ...named(editor),
                mode: "set",
                value: "vim",
                release: editor.package.version,
            }),
        );

        // resolve the set value and the other setting's default
        expect([
            await SettingValue.resolve(Snapshot.live(storage.database), editor, { scope: alice }),
            await SettingValue.resolve(Snapshot.live(storage.database), lineNumbers, {
                scope: alice,
            }),
        ]).toEqual(["vim", true]);
    },
);

test.each(TEST_DIALECTS)(
    "read a setting of days as a retention in a space that sets it, and its default in a space that does not and in another scope type, on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());

        // keep Bob's space's history a week
        await new AccessFixture(storage.database).replicateScope(
            principal.space.reference(Scope.universe.id, space),
        );
        await storage.database.insert(setting.table).values({
            id: setting.generateId(),
            scope: space,
            packageId: history.package.id,
            name: history.name,
            mode: "set",
            value: 7,
            release: history.package.version,
            createdAt: 1000,
            updatedAt: 1000,
        });

        // read the space's week, the default in another space, and the default in Alice's personal scope
        const retention = SettingValue.retention(history);
        expect([
            await retention(space, storage.database),
            await retention("space-019f5530-8000-7000-8000-000000000099", storage.database),
            await retention(alice, storage.database),
        ]).toEqual([{ days: 7 }, { days: 30 }, { days: 30 }]);
    },
);

test.each(TEST_DIALECTS)(
    "follow a scope's values for a setting and yield them again after a change on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());
        const created = await storage.call(
            setting.call("create", {
                scope: alice,
                ...named(editor),
                mode: "set",
                value: "vim",
                release: "2026.9.0",
            }),
        );

        // read the values before and after a change
        const stopping = new AbortController();
        onTestFinished(() => stopping.abort());
        const followed = SettingValue.follow(
            storage.database,
            [editor.reference],
            { scope: alice },
            stopping.signal,
        );
        const first = await followed.next();
        await storage.call(
            setting.call("update", { scope: alice, id: created.id, value: "standard" }),
        );
        const second = await followed.next();
        expect(
            [first, second].map((step) =>
                step.done === true ? [] : step.value.values.map((row) => row.value),
            ),
        ).toEqual([["vim"], ["standard"]]);
    },
);
