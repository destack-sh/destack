import { DeclarationReference } from "@destack/package/declare";
import { expect, test, refusal } from "@destack/test";
import { schema } from "@destack/schema";
import { Expression } from "@destack/db";
import { PackageId } from "@destack/package";
import { defineSetting } from "../src/declare/index.ts";
import {
    compareSetting,
    describeSetting,
    settingVocabulary,
    SettingDescription,
} from "../src/inspect/index.ts";
import type { DeviceSettingValue, SettingValue } from "../src/object/index.ts";
import { SettingResolution, SettingSelection } from "../src/setting/index.ts";
import { Setting } from "../src/setting/index.ts";
import {
    account,
    alice,
    chain,
    deviceSource,
    installation,
    laptop,
    named,
    personal,
    required,
    selection,
    source,
    space,
} from "./fixture/index.ts";
import { editor, keybindings, notes, release } from "./fixture/setting/index.ts";

/** The package whose releases the entries declare. */
const PACKAGE = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-00000000c001"),
    name: "@destack/compare-fixture",
};

/** A release's manifest entry of a declaration, its description read back from JSON. */
const entry = (description: unknown, version: string) => ({
    description: schema.jsonObject().parse(JSON.parse(JSON.stringify(description))),
    symbol: {
        package: { ...PACKAGE, version },
        symbol: { module: "src/index.ts", name: "declared" },
    },
});

/** The resolution of the editor mode to its default for the fixture's selection. */
const standard = {
    setting: editor.reference,
    selection,
    value: "standard",
    sources: [{ kind: "default", package: notes }],
    overridden: [],
    enforcement: "ordinary",
};

test("resolve personal, device and required values independently of their order", async () => {
    // take the device's value over the personal one and a recommendation, in either order
    const recommendation: SettingValue = { ...required, mode: "recommend" };
    const ordinary = {
        ...standard,
        sources: [deviceSource(laptop)],
        overridden: [{ kind: "default", package: notes }, source(recommendation), source(personal)],
    };
    expect([
        editor.resolve(selection, [personal, recommendation], chain, [laptop]),
        editor.resolve(selection, [recommendation, personal], chain, [laptop]),
    ]).toEqual([ordinary, ordinary]);

    // expose the personal value once the device's value goes
    expect(editor.resolve(selection, [personal], chain)).toEqual({
        ...standard,
        value: "vim",
        sources: [source(personal)],
        overridden: [{ kind: "default", package: notes }],
    });

    // take a requirement over every ordinary value, the device's included
    expect(editor.resolve(selection, [personal, required], chain, [laptop])).toEqual({
        ...standard,
        value: "vim",
        enforcement: "required",
        sources: [source(required)],
        overridden: [{ kind: "default", package: notes }, source(personal), deviceSource(laptop)],
    });
});

test("refuse values set in other scopes or outside the chain, and disagreeing requirements", async () => {
    // refuse another user's value, and resolve the default for an anonymous visitor
    const anonymous = SettingSelection.parse({ scope: null });
    const misplaced = [
        "BAD_REQUEST",
        "INVALID_PLACEMENT",
        "the resolution got a value set in another scope",
    ];
    expect([
        await refusal(() => editor.resolve(selection, [{ ...personal, scope: "user-bob" }], chain)),
        await refusal(() => editor.resolve(anonymous, [personal], chain)),
        await refusal(() =>
            editor.resolve(selection, [{ ...required, scope: "space-elsewhere" }], chain),
        ),
    ]).toEqual([
        misplaced,
        misplaced,
        [
            "BAD_REQUEST",
            "INVALID_PLACEMENT",
            "the resolution got a value from outside the selection's scope chain",
        ],
    ]);
    expect(editor.resolve(anonymous, [], chain)).toEqual({ ...standard, selection: anonymous });

    // refuse disagreeing requirements and a selection of another scope than the setting's
    expect([
        await refusal(() =>
            editor.resolve(selection, [required, { ...required, value: "standard" }], chain),
        ),
        await refusal(() => editor.resolve(SettingSelection.parse({ scope: space }), [], chain)),
    ]).toEqual([
        ["CONFLICT", "CONFLICTING_VALUES", "required setting values conflict"],
        ["BAD_REQUEST", "INVALID_PLACEMENT", "setting scope does not match the selected scope"],
    ]);
});

test("resolve each key of a merged setting from its own nearest placement, requirements fixing theirs", async () => {
    // bind one command personally, another on the device, and require a third in the space
    const bindings = { ...personal, ...named(keybindings) };
    const mine = { ...bindings, value: { "note.archive": "mod+e", "note.pin": "mod+p" } };
    const local = { ...laptop, ...named(keybindings), value: { "note.search": "mod+k" } };
    const fixed = { ...required, ...named(keybindings), value: { "note.pin": null } };
    const resolved = keybindings.resolve(selection, [fixed, mine], chain, [local]);

    // take every key from the placement deciding it, the device override leaving the personal keys
    expect(resolved).toEqual({
        setting: keybindings.reference,
        selection,
        value: { "note.archive": "mod+e", "note.pin": null, "note.search": "mod+k" },
        sources: [
            { kind: "default", package: notes },
            source(mine),
            deviceSource(local),
            source(fixed),
        ],
        overridden: [],
        enforcement: "required",
        keys: {
            "note.archive": {
                source: source(mine),
                overridden: [{ kind: "default", package: notes }],
                enforcement: "ordinary",
            },
            "note.pin": {
                source: source(fixed),
                overridden: [source(mine)],
                enforcement: "required",
            },
            "note.search": { source: deviceSource(local), overridden: [], enforcement: "ordinary" },
        },
    });

    // refuse equally placed values disagreeing on a key
    expect(
        await refusal(() =>
            keybindings.resolve(
                selection,
                [mine, { ...mine, id: required.id, value: { "note.archive": "mod+r" } }],
                chain,
            ),
        ),
    ).toEqual([
        "CONFLICT",
        "CONFLICTING_VALUES",
        "multiple values of note.archive have the same precedence",
    ]);
});

test("compare required structured values independently of key order", async () => {
    const layout = defineSetting(
        {
            ...editor.definition,
            name: "layout",
            schema: schema.object({ width: schema.number(), compact: schema.boolean() }),
            default: { width: 80, compact: false },
        },
        { package: notes },
    );
    const requirement: SettingValue = {
        ...required,
        ...named(layout),
        value: { width: 80, compact: true },
    };
    const equivalent: SettingValue = {
        ...requirement,
        id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000010"),
        scope: account,
        value: { compact: true, width: 80 },
    };
    expect(layout.resolve(selection, [equivalent, requirement], chain)).toEqual({
        ...standard,
        setting: layout.reference,
        value: { width: 80, compact: true },
        sources: [source(requirement), source(equivalent)],
        overridden: [{ kind: "default", package: notes }],
        enforcement: "required",
    });
});

test("describe a setting and read the settings a build declares", async () => {
    // describe the value schema as JSON Schema
    const layout = defineSetting(
        {
            ...editor.definition,
            name: "layout",
            schema: schema.object({
                width: schema.number().int().min(1),
                compact: schema.boolean(),
            }),
            default: { width: 80, compact: false },
        },
        { package: notes },
    );
    expect(SettingDescription.parse(JSON.parse(JSON.stringify(describeSetting(editor))))).toEqual({
        package: notes,
        name: "editor.mode",
        title: "Editor mode",
        description: "Keyboard behavior in the note editor.",
        schema: {
            $schema: "https://json-schema.org/draft/2020-12/schema",
            type: "string",
            enum: ["standard", "vim"],
        },
        default: "standard",
        scope: "user",
        overrides: ["space", "installation", "device"],
        apply: "immediate",
    });
    expect(settingVocabulary(editor)).toEqual({
        "editor.mode": { scope: "user" },
    });

    // read both settings back, validating values by their described schemas
    const settings = await Setting.list(await release([editor, layout]));
    const read = Setting.find(settings, layout.reference);
    const [first] = settings;
    if (first === undefined) {
        throw new TypeError("the build declares no settings");
    }
    expect([
        settings.map(describeSetting),
        first.definition.schema.safeParse("vim").success,
        first.definition.schema.safeParse("emacs").success,
        read.definition.schema.safeParse({ width: 100, compact: true }).success,
        read.definition.schema.safeParse({ width: 0, compact: true }).success,
    ]).toEqual([[describeSetting(editor), describeSetting(layout)], true, false, true, false]);

    // refuse a setting the build does not declare
    expect(
        await refusal(() => Setting.find(settings, { ...editor.reference, name: "theme" })),
    ).toEqual(["NOT_FOUND", "UNDECLARED", `setting ${notes.id}/theme is not declared`]);
});

test("resolve space settings per installation and machine settings per machine", async () => {
    // resolve one space setting independently for two installations
    const template = defineSetting(
        { ...editor.definition, name: "template", scope: "space", overrides: ["installation"] },
        { package: notes },
    );
    const placed = schema
        .identifier("installation")
        .parse("installation-019f5530-8000-7000-8000-000000000004");
    const first = SettingSelection.parse({ scope: space, installation: placed });
    const second = SettingSelection.parse({
        scope: space,
        installation: "installation-019f5530-8000-7000-8000-000000000014",
    });
    const value: SettingValue = {
        ...personal,
        ...named(template),
        scope: space,
        installation: placed,
    };
    expect([
        template.resolve(first, [value], chain),
        template.resolve(second, [value], chain),
    ]).toEqual([
        {
            ...standard,
            setting: template.reference,
            selection: first,
            value: "vim",
            sources: [source(value)],
            overridden: [{ kind: "default", package: notes }],
        },
        { ...standard, setting: template.reference, selection: second },
    ]);

    // resolve a device setting from the value its device stores, and refuse a value placed in a scope
    const cache = defineSetting(
        {
            ...editor.definition,
            name: "cacheDirectory",
            scope: "device",
            overrides: [],
            schema: schema.string().min(1),
            default: "/cache",
            apply: "restart",
        },
        { package: notes },
    );
    const device = "device-019f5530-8000-7000-8000-000000000015";
    const path: DeviceSettingValue = {
        ...laptop,
        ...named(cache),
        value: "/Users/alice/cache",
    };
    const elsewhere: SettingValue = { ...personal, ...named(cache), scope: device };
    const onDevice = SettingSelection.parse({ scope: device });
    expect(cache.resolve(onDevice, [], [device], [path])).toEqual({
        setting: cache.reference,
        selection: onDevice,
        value: "/Users/alice/cache",
        sources: [deviceSource(path)],
        overridden: [{ kind: "default", package: notes }],
        enforcement: "ordinary",
    });
    expect(
        await refusal(() =>
            cache.resolve(
                SettingSelection.parse({ scope: "device-019f5530-8000-7000-8000-000000000016" }),
                [elsewhere],
                [device],
            ),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "INVALID_PLACEMENT",
        "the resolution got a value set in another scope",
    ]);
});

test("refuse placements and writes a setting does not permit", async () => {
    // refuse overrides the setting does not declare, and set values outside its scope
    const plain = defineSetting({ ...editor.definition, overrides: [] }, { package: notes });
    const unsupported = [
        "BAD_REQUEST",
        "INVALID_PLACEMENT",
        "setting value uses an unsupported setting override",
    ];
    expect([
        await refusal(() => plain.requirePlacement({ installation, mode: "set" }, "declared")),
        await refusal(() => plain.resolve(selection, [], chain, [laptop])),
        await refusal(() => editor.requirePlacement({ mode: "set" }, "enclosing")),
        await refusal(() => editor.requirePlacement({ mode: "recommend" }, "declared")),
        await refusal(() =>
            editor.requirePlacement({ space, installation, mode: "set" }, "declared"),
        ),
    ]).toEqual([
        unsupported,
        [
            "BAD_REQUEST",
            "INVALID_PLACEMENT",
            `devices cannot store values of ${DeclarationReference.key(plain.reference)}`,
        ],
        [
            "BAD_REQUEST",
            "INVALID_PLACEMENT",
            "setting value outside its declared scope must be a recommendation or a requirement",
        ],
        ["BAD_REQUEST", "INVALID_PLACEMENT", "setting value in its declared scope must be set"],
        [
            "BAD_REQUEST",
            "INVALID_PLACEMENT",
            "setting value has both an installation and its space",
        ],
    ]);

    // accept a current write in its scope, and refuse one of a later release or with an invalid value
    const write = { mode: "set" as const, value: "vim", release: "2026.9.0" };
    expect([
        await refusal(() => editor.requireWrite(write, alice)),
        await refusal(() => editor.requireWrite({ ...write, release: "2026.10.0" }, alice)),
        await refusal(() => editor.requireWrite({ ...write, value: "emacs" }, alice)),
        await refusal(() => editor.requireWrite({ ...write, mode: "recommend" }, space)),
    ]).toEqual([
        "done",
        [
            "BAD_REQUEST",
            "INVALID_VALUE",
            "setting value is at release 2026.10.0, but its declaration is at 2026.9.0",
        ],
        ["BAD_REQUEST", "INVALID_VALUE", "setting value does not match its declaration"],
        "done",
    ]);
});

test("skip stored values a changed schema rejects, falling through to the next source", () => {
    // skip Alice's value for the recommendation and the default, listing it as invalid
    const upgraded = defineSetting(
        { ...editor.definition, schema: schema.enum(["standard", "emacs"]), default: "standard" },
        { package: { ...notes, version: "2026.10.0" } },
    );
    const recommendation: SettingValue = { ...required, mode: "recommend", value: "emacs" };
    expect([
        upgraded.resolve(selection, [personal, recommendation], chain),
        upgraded.resolve(selection, [personal], chain),
    ]).toEqual([
        {
            ...standard,
            setting: upgraded.reference,
            value: "emacs",
            sources: [source(recommendation), source(personal, "invalid")],
            overridden: [{ kind: "default", package: { ...notes, version: "2026.10.0" } }],
        },
        {
            ...standard,
            setting: upgraded.reference,
            sources: [
                { kind: "default", package: { ...notes, version: "2026.10.0" } },
                source(personal, "invalid"),
            ],
        },
    ]);

    // skip the device's value the changed schema rejects too
    const emacs = { ...laptop, value: "vim" };
    expect(upgraded.resolve(selection, [], chain, [emacs]).sources).toEqual([
        { kind: "default", package: { ...notes, version: "2026.10.0" } },
        deviceSource(emacs, "invalid"),
    ]);

    // find the invalid values at their placement and on the device, and nothing where no value is placed
    const resolution = upgraded.resolve(selection, [personal], chain);
    const local = upgraded.resolve(selection, [personal], chain, [emacs]);
    expect([
        SettingResolution.observed(resolution, { scope: alice }),
        SettingResolution.observed(resolution, { scope: alice, installation }),
        SettingResolution.observed(resolution, "device"),
        SettingResolution.observed(local, "device"),
    ]).toEqual([
        { id: personal.id, revision: personal.revision },
        null,
        null,
        { id: emacs.id, revision: emacs.revision },
    ]);
});

test("convert stored values of earlier releases, and skip values of later releases and ones no conversion reaches", () => {
    // convert the editor mode to a named keymap in the setting's release
    const keymap = defineSetting(
        {
            ...editor.definition,
            schema: schema.object({ keymap: schema.enum(["standard", "vim"]) }),
            default: { keymap: "standard" },
            convert: { "2026.9.0": Expression.object({ keymap: Expression.column("value") }) },
        },
        { package: notes },
    );
    const unconverted = new Setting(notes, { ...keymap.definition, convert: {} });
    const earlier: SettingValue = { ...personal, release: "2026.8.0" };
    const current: SettingValue = { ...personal, value: { keymap: "vim" } };
    const expected = {
        ...standard,
        setting: keymap.reference,
        value: { keymap: "vim" },
        sources: [source(earlier)],
        overridden: [{ kind: "default", package: notes }],
    };
    expect([
        keymap.resolve(selection, [earlier], chain),
        keymap.resolve(selection, [{ ...earlier, release: "2026.10.0" }], chain).sources,
        unconverted.resolve(selection, [earlier], chain).sources,
        keymap.resolve(selection, [current], chain).value,
    ]).toEqual([
        expected,
        [
            { kind: "default", package: notes },
            source({ ...earlier, release: "2026.10.0" }, "invalid"),
        ],
        [{ kind: "default", package: notes }, source(earlier, "invalid")],
        { keymap: "vim" },
    ]);

    // refuse a conversion keyed by a release after the setting's release
    expect(() =>
        defineSetting(
            { ...editor.definition, convert: { "2026.10.0": Expression.column("value") } },
            { package: notes },
        ),
    ).toThrow(
        new TypeError(
            "conversion of editor.mode is keyed by 2026.10.0, after its release 2026.9.0",
        ),
    );
});

test("resolve a value for its consuming package, and rank recommendations by the nearness of their scope", async () => {
    // apply a value for the consuming package in any space, and nowhere else
    const shared = defineSetting(
        {
            ...editor.definition,
            scope: "user",
            overrides: ["package", "space", "installation", "device"],
        },
        { package: notes },
    );
    const homeId = PackageId.parse("package-019f5530-8000-7000-8000-000000000020");
    const home = SettingSelection.parse({ ...selection, package: homeId });
    const elsewhere = SettingSelection.parse({
        scope: alice,
        package: homeId,
        space: "space-019f5530-8000-7000-8000-000000000021",
    });
    const app: SettingValue = { ...personal, package: homeId };
    const applied = {
        ...standard,
        setting: shared.reference,
        value: "vim",
        sources: [source(app)],
        overridden: [{ kind: "default", package: notes }],
    };
    expect([
        shared.resolve(home, [app], chain),
        shared.resolve(elsewhere, [app], chain),
        shared.resolve(selection, [app], chain),
    ]).toEqual([
        { ...applied, selection: home },
        { ...applied, selection: elsewhere },
        { ...standard, setting: shared.reference },
    ]);

    // take the space's recommendation over the account's disagreeing one
    const nearer: SettingValue = { ...required, mode: "recommend" };
    const farther: SettingValue = {
        ...nearer,
        id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000022"),
        scope: account,
        value: "standard",
    };
    expect(shared.resolve(home, [farther, nearer], chain)).toEqual({
        ...applied,
        selection: home,
        sources: [source(nearer)],
        overridden: [{ kind: "default", package: notes }, source(farther)],
    });

    // refuse disagreeing recommendations of one scope
    const sibling: SettingValue = {
        ...nearer,
        id: schema.identifier("setting").parse("setting-019f5530-8000-7000-8000-000000000024"),
        value: "standard",
    };
    expect(await refusal(() => shared.resolve(home, [nearer, sibling], chain))).toEqual([
        "CONFLICT",
        "CONFLICTING_VALUES",
        "multiple setting values have the same precedence",
    ]);
});

test("plan a setting's value change between releases: safe widenings, converted narrowings, and a refused unconverted narrowing", () => {
    // describe the editor mode, a wider mode, and a keymap narrowing it
    const version = "2026.9.0";
    const before = describeSetting(editor);
    const wider = describeSetting(
        defineSetting(
            { ...editor.definition, schema: schema.enum(["standard", "vim", "emacs"]) },
            { package: notes },
        ),
    );
    const narrower = defineSetting(
        { ...editor.definition, schema: schema.enum(["vim"]), default: "vim" },
        { package: notes },
    );
    const converted = describeSetting(
        defineSetting(
            {
                ...narrower.definition,
                convert: { [version]: Expression.literal("vim") },
            },
            { package: notes },
        ),
    );
    const outcome = (after: SettingDescription) => {
        try {
            return compareSetting(entry(before, "2026.8.0"), entry(after, version)).steps;
        } catch (error) {
            if (!(error instanceof Error)) {
                throw error;
            }

            return error.message;
        }
    };

    // widen safely, convert a narrowing with a conversion, and refuse one without
    expect([
        outcome(before),
        outcome(wider),
        outcome(converted),
        outcome(describeSetting(narrower)),
    ]).toEqual([
        [],
        [
            {
                action: "update",
                target: "setting/editor.mode",
                risk: "safe",
                detail: "wider values",
            },
        ],
        [
            {
                action: "convert",
                target: "setting/editor.mode",
                risk: "fallible",
                detail: `convert values to ${version}`,
            },
        ],
        "setting/editor.mode: declare a conversion for 2026.9.0",
    ]);
});

test("read a setting's value among an installation's resolved settings, refusing an invalid or unresolved one", async () => {
    // read the resolved value by the setting's key
    const key = DeclarationReference.key(editor.reference);
    const read = editor.read({ [key]: "vim" });

    // refuse a value the schema rejects and a setting the host left out
    expect(read).toBe("vim");
    expect(() => editor.read({ [key]: "emacs" })).toThrow(schema.Error);
    expect(await refusal(() => editor.read({}))).toEqual([
        "NOT_FOUND",
        "UNDECLARED",
        `the installation's machine did not resolve ${key}`,
    ]);
});
