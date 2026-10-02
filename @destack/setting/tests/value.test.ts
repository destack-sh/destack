import { asc } from "@destack/db";
import { principal } from "@destack/access";
import { Subject, Scope } from "@destack/sync";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, test } from "@destack/test";
import { reportError } from "@destack/service/server";
import { setting, type SettingValue } from "../src/object/index.ts";
import { alice, named, selection } from "./fixture/value.ts";
import { editor, lineNumbers } from "./fixture/settings/index.ts";
import { Storage } from "./fixture/storage.ts";

/** Read a refused call's failure as the caller receives it: its code and message. */
async function refusal(call: Promise<unknown>): Promise<{ code: string; message: string }> {
    const error = reportError(
        await call.then(
            () => undefined,
            (error: unknown) => error,
        ),
    );

    return { code: error.code, message: error.message };
}

test.each(TEST_DIALECTS)(
    "write declared values at their placements and refuse invalid, undeclared and misplaced ones on %s",
    async (dialect) => {
        const storage = await Storage.open(dialect);
        onTestFinished(() => storage.close());
        const laptop = await storage.register("laptop");
        const call = (name: string, input: Readonly<Record<string, unknown>>) =>
            storage.call(setting, name, alice, input) as Promise<SettingValue>;
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
            device: laptop.id,
            value: "standard",
        });
        const changed = await call("update", { id: override.id, value: "vim" });
        const rows = await storage.database
            .select()
            .from(setting.table)
            .orderBy(asc(setting.table.revision), asc(setting.table.device));
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
                device: null,
                value: "vim",
            },
            {
                ...written,
                id: override.id,
                createdAt: override.createdAt,
                updatedAt: changed.updatedAt,
                revision: 2,
                installation: selection.installation,
                device: laptop.id,
                value: "vim",
            },
        ]);

        // refuse a second value at the same placement, values the setting rejects on creation
        //  and update, a value of a later release, an undeclared setting, and misplaced values
        const mismatch = {
            code: "BAD_REQUEST",
            message: "setting value does not match its declaration",
        };
        expect([
            await refusal(create({})),
            await refusal(create({ value: "emacs" })),
            await refusal(call("update", { id: base.id, value: "emacs" })),
            await refusal(create({ release: "2026.10.0" })),
            await refusal(create({ name: "editor.theme" })),
            await refusal(create({ mode: "recommend" })),
            await refusal(create({ ...named(lineNumbers), space: selection.space, value: false })),
        ]).toEqual([
            { code: "CONFLICT", message: "a record with the same unique key exists" },
            mismatch,
            mismatch,
            {
                code: "BAD_REQUEST",
                message: "setting value is at release 2026.10.0, its declaration at 2026.9.0",
            },
            {
                code: "NOT_FOUND",
                message: `setting ${editor.reference.packageId}/editor.theme is not declared`,
            },
            { code: "BAD_REQUEST", message: "setting value in its declared scope must be set" },
            {
                code: "BAD_REQUEST",
                message: "setting value uses an unsupported setting override",
            },
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
        device: selection.device,
    });
    expect(await refusal(unknown)).toEqual({
        code: "CONFLICT",
        message: "the change would leave a reference to a missing record",
    });
});
