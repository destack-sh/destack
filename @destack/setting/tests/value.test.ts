import { asc } from "@destack/db";
import { principal } from "@destack/access";
import { Subject, Scope } from "@destack/sync";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { setting } from "../src/object/index.ts";
import { alice, named, selection } from "./fixture/value.ts";
import { editor, lineNumbers } from "./fixture/settings/index.ts";
import { Storage } from "./fixture/storage.ts";

test.each(TEST_DIALECTS)(
    "write declared values at their placements and refuse invalid, undeclared and misplaced ones on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());
        const laptop = await storage.register("laptop");
        const call = async (name: "create" | "update", input: Readonly<Record<string, unknown>>) =>
            await storage.call(setting, name, alice, input);
        const create = (input: Readonly<Record<string, unknown>>) =>
            call("create", {
                ...named(editor),
                mode: "set",
                value: "vim",
                release: "2026.9.0",
                ...input,
            });

        // write a personal value, an override on the laptop in Bob's installation, then change it
        const base = await create({});
        const override = await create({
            installation: selection.installation,
            deviceId: laptop.id,
            value: "standard",
        });
        const changed = await call("update", { id: override.id, value: "vim" });
        const rows = await storage.database
            .select()
            .from(setting.table)
            .orderBy(asc(setting.table.revision), asc(setting.table.deviceId));
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
                deviceId: null,
                value: "vim",
            },
            {
                ...written,
                id: override.id,
                createdAt: override.createdAt,
                updatedAt: changed.updatedAt,
                revision: 2,
                installation: selection.installation,
                deviceId: laptop.id,
                value: "vim",
            },
        ]);

        // refuse a second value at the same placement, values the setting rejects on creation
        //  and update, a value of a later release, an undeclared setting, and misplaced values
        const mismatch = ["BAD_REQUEST", "setting value does not match its declaration"];
        expect([
            await refusal(create({})),
            await refusal(create({ value: "emacs" })),
            await refusal(call("update", { id: base.id, value: "emacs" })),
            await refusal(create({ release: "2026.10.0" })),
            await refusal(create({ name: "editor.theme" })),
            await refusal(create({ mode: "recommend" })),
            await refusal(create({ ...named(lineNumbers), space: selection.space, value: false })),
        ]).toEqual([
            ["DUPLICATE", "a record with the same unique key exists"],
            mismatch,
            mismatch,
            ["BAD_REQUEST", "setting value is at release 2026.10.0, its declaration at 2026.9.0"],
            ["NOT_FOUND", `setting ${editor.reference.packageId}/editor.theme is not declared`],
            ["BAD_REQUEST", "setting value in its declared scope must be set"],
            ["BAD_REQUEST", "setting value uses an unsupported setting override"],
        ]);
    },
);

test.each(TEST_DIALECTS)("refuse values for devices no one registered on %s", async (dialect) => {
    const storage = await Storage.open(dialect);
    onTestFinished(() => storage.close());

    // refuse a device no one registered
    const unknown = storage.call(setting, "create", alice, {
        ...named(editor),
        mode: "set",
        value: "vim",
        release: editor.package.version,
        deviceId: selection.deviceId,
    });
    expect(await refusal(unknown)).toEqual([
        "BROKEN_REFERENCE",
        "the change would leave a reference to a missing record",
    ]);
});
