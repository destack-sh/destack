import { asc, Snapshot } from "@destack/db";
import { principal } from "@destack/access";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { Duration } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { setting, SettingValue } from "../src/object/index.ts";
import settingPackage from "../src/package.ts";
import { alice, aliceUser, installation, named, selection, space } from "./fixture/value.ts";
import { editor, history, lineNumbers } from "./fixture/setting/index.ts";
import { openSettings } from "./fixture/store.ts";

/** The milliseconds in a day. */
const DAY = Duration.milliseconds({ days: 1 });

test("write declared values at their placements and refuse invalid, undeclared and misplaced ones", async () => {
    // serve Alice's personal scope, writing as her
    await using served = await openSettings({ scope: aliceUser });
    const values = served.as(alice).client(settingPackage).setting;
    const placed = {
        scope: alice,
        ...named(editor),
        mode: "set",
        release: editor.package.version,
    } as const;
    const create = (input: Partial<Parameters<typeof values.create>[0]> = {}) =>
        values.create({
            ...placed,
            value: "vim",
            requestId: RequestId.create(Date.now()),
            ...input,
        });

    // write a personal value and an override in Bob's installation, and change it
    const base = await create();
    const override = await create({ installation, value: "standard" });
    const changed = await values.update({
        scope: alice,
        id: override.id,
        value: "vim",
        requestId: RequestId.create(Date.now()),
    });
    const rows = await served.database
        .select()
        .from(setting.table)
        .orderBy(asc(setting.table.revision), asc(setting.table.installation));
    const stored = {
        ...ObjectReference.columns("createdBy", aliceUser),
        ...ObjectReference.columns("updatedBy", aliceUser),
        ...placed,
        package: null,
        space: null,
        revision: 1,
        tags: {},
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
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

    // refuse duplicate, invalid, later, undeclared and misplaced values as the client receives them
    const mismatch = [
        "BAD_REQUEST",
        "INVALID_VALUE",
        "setting value does not match its declaration",
    ];
    expect([
        await refusal(create()),
        await refusal(create({ value: "emacs" })),
        await refusal(
            values.update({
                scope: alice,
                id: base.id,
                value: "emacs",
                requestId: RequestId.create(Date.now()),
            }),
        ),
        await refusal(create({ release: "2026.10.0" })),
        await refusal(create({ name: "editor.theme" })),
        await refusal(create({ mode: "recommend" })),
        await refusal(create({ ...named(lineNumbers), space, value: false })),
    ]).toEqual([
        [
            "CONFLICT",
            "DUPLICATE",
            "another setting has the same packageId, name, scope, package, space, and installation",
        ],
        mismatch,
        mismatch,
        [
            "BAD_REQUEST",
            "INVALID_VALUE",
            "setting value is at release 2026.10.0, its declaration at 2026.9.0",
        ],
        [
            "NOT_FOUND",
            "UNDECLARED",
            `setting ${editor.reference.packageId}/editor.theme is not declared`,
        ],
        ["BAD_REQUEST", "INVALID_PLACEMENT", "setting value in its declared scope must be set"],
        ["BAD_REQUEST", "INVALID_PLACEMENT", "setting value uses an unsupported setting override"],
    ]);
});

test("resolve a setting from the values placed along a scope's chain, and its default without one", async () => {
    // set a personal editor mode and leave line numbers unset
    await using served = await openSettings({ scope: aliceUser });
    await served
        .as(alice)
        .client(settingPackage)
        .setting.create({
            scope: alice,
            ...named(editor),
            mode: "set",
            value: "vim",
            release: editor.package.version,
            requestId: RequestId.create(Date.now()),
        });

    // resolve the set value and the other setting's default
    const view = Snapshot.live(served.database);
    expect([
        await SettingValue.resolve(view, editor, { scope: alice }),
        await SettingValue.resolve(view, lineNumbers, { scope: alice }),
    ]).toEqual(["vim", true]);
});

test("read the retention a package's setting of days gives a space that sets it, and its default in a space that does not and in another scope type", async () => {
    // set one space's history to a week as its owner, beside a space and Alice's scope setting nothing
    await using kept = await openSettings();
    await using other = await openSettings();
    await using personal = await openSettings({ scope: aliceUser });
    await kept
        .as(kept.scope)
        .client(settingPackage)
        .setting.create({
            scope: kept.scope,
            ...named(history),
            mode: "set",
            value: 7,
            release: history.package.version,
            requestId: RequestId.create(Date.now()),
        });

    // read the space's week, the default in the other space, and the default in Alice's personal scope
    const { definition } = history;
    expect([
        await SettingValue.retention(kept.database)(definition, history.package, kept.scope),
        await SettingValue.retention(other.database)(definition, history.package, other.scope),
        await SettingValue.retention(personal.database)(definition, history.package, alice),
    ]).toEqual([7 * DAY, 30 * DAY, 30 * DAY]);
});

test("follow a scope's values for a setting and yield them again after a change", async () => {
    // set Alice's editor mode
    await using served = await openSettings({ scope: aliceUser });
    const values = served.as(alice).client(settingPackage).setting;
    const created = await values.create({
        scope: alice,
        ...named(editor),
        mode: "set",
        value: "vim",
        release: editor.package.version,
        requestId: RequestId.create(Date.now()),
    });

    // read the values before and after a change, stopping the follower with the test
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    const followed = SettingValue.follow(
        served.database,
        [editor.reference],
        { scope: alice },
        stopping.signal,
    );
    const first = await followed.next();
    const changed = await values.update({
        scope: alice,
        id: created.id,
        value: "standard",
        requestId: RequestId.create(Date.now()),
    });
    const second = await followed.next();

    // yield the stored row, then its change
    expect([first, second].map((step) => (step.done === true ? [] : step.value.values))).toEqual([
        [created],
        [changed],
    ]);
});
