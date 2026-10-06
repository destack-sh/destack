import { asc } from "@destack/db";
import type { JsonObject } from "@destack/schema";
import { principal } from "@destack/access";
import { Subject, Scope } from "@destack/sync";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { setting, SettingValue } from "../src/object/index.ts";
import { alice, installation, named, selection, space } from "./fixture/value.ts";
import { editor, lineNumbers } from "./fixture/setting/index.ts";
import { Storage } from "./fixture/storage.ts";

test.each(TEST_DIALECTS)(
    "write declared values at their placements and refuse invalid, undeclared and misplaced ones on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());
        const laptop = await storage.register("laptop");
        const call = async (name: "create" | "update", input: JsonObject) =>
            await storage.call(setting, name, alice, input);
        const create = (input: JsonObject) =>
            call("create", {
                ...named(editor),
                mode: "set",
                value: "vim",
                release: "2026.9.0",
                ...input,
            });

        // write a personal value and an override on the laptop in Bob's installation, and change it
        const base = await create({});
        const override = await create({
            installation,
            clientId: laptop.id,
            value: "standard",
        });
        const changed = await call("update", { id: override.id, value: "vim" });
        const rows = await storage.database
            .select()
            .from(setting.table)
            .orderBy(asc(setting.table.revision), asc(setting.table.clientId));
        const author = Subject.key(principal.user.reference(Scope.universe.id, alice));
        const written = {
            createdBy: author,
            updatedBy: author,
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
                ...written,
                id: base.id,
                createdAt: base.createdAt,
                updatedAt: base.updatedAt,
                installation: null,
                clientId: null,
                value: "vim",
            },
            {
                ...written,
                id: override.id,
                createdAt: override.createdAt,
                updatedAt: changed.updatedAt,
                revision: 2,
                installation: selection.installation,
                clientId: laptop.id,
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
            ["DUPLICATE", "a record with the same unique key exists"],
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
        await storage.call(setting, "create", alice, {
            ...named(editor),
            mode: "set",
            value: "vim",
            release: editor.package.version,
        });

        // resolve the set value and the other setting's default
        expect([
            await SettingValue.resolve(storage.database, editor, { scope: alice }),
            await SettingValue.resolve(storage.database, lineNumbers, { scope: alice }),
        ]).toEqual(["vim", true]);
    },
);

test.each(TEST_DIALECTS)(
    "follow a scope's values for a setting and yield them again after a change on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());
        const created = await storage.call(setting, "create", alice, {
            ...named(editor),
            mode: "set",
            value: "vim",
            release: "2026.9.0",
        });

        // read the values before and after a change
        const stopping = new AbortController();
        onTestFinished(() => stopping.abort());
        const followed = SettingValue.follow(
            storage.database,
            [editor.reference],
            alice,
            stopping.signal,
        );
        const first = await followed.next();
        await storage.call(setting, "update", alice, { id: created.id, value: "standard" });
        const second = await followed.next();
        expect(
            [first, second].map((step) =>
                step.done === true ? [] : step.value.values.map((row) => row.value),
            ),
        ).toEqual([["vim"], ["standard"]]);
    },
);
