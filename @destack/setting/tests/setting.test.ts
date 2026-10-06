import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Expression } from "@destack/db";
import { PackageId } from "@destack/package";
import { Scope } from "@destack/sync";
import { defineSetting } from "../src/declare/index.ts";
import { SettingError } from "../src/error/index.ts";
import {
    compareSetting,
    describeSetting,
    settingVocabulary,
    SettingDescription,
} from "../src/inspect/index.ts";
import type { SettingValue } from "../src/object/index.ts";
import { SettingResolution, SettingSelection } from "../src/setting/index.ts";
import { Setting, SettingCatalog } from "../src/setting/index.ts";
import {
    account,
    alice,
    chain,
    clientId,
    installation,
    named,
    override,
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
    description: schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(JSON.stringify(description))),
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

/** Report a setting failure as its code and message, or as accepted. */
function failure(run: () => unknown): { code: string; message: string } | "accepted" {
    try {
        run();

        return "accepted";
    } catch (error) {
        if (!(error instanceof SettingError)) {
            throw error;
        }

        return { code: error.code, message: error.message };
    }
}

test("resolve personal, client and required values independently of their order", () => {
    // take the client override over the personal value, in either order
    const ordinary = {
        ...standard,
        sources: [source(override)],
        overridden: [{ kind: "default", package: notes }, source(personal)],
    };
    expect([
        editor.resolve(selection, [override, personal], chain),
        editor.resolve(selection, [personal, override], chain),
    ]).toEqual([ordinary, ordinary]);

    // expose the personal value once the override goes
    expect(editor.resolve(selection, [personal], chain)).toEqual({
        ...standard,
        value: "vim",
        sources: [source(personal)],
        overridden: [{ kind: "default", package: notes }],
    });

    // take a requirement over every ordinary value
    expect(editor.resolve(selection, [override, personal, required], chain)).toEqual({
        ...standard,
        value: "vim",
        enforcement: "required",
        sources: [source(required)],
        overridden: [...ordinary.overridden, ...ordinary.sources],
    });
});

test("refuse values set in other scopes or outside the chain, and disagreeing requirements", () => {
    // refuse another user's value, and resolve the default for an anonymous visitor
    const anonymous = SettingSelection.parse({ scope: null });
    const misplaced = {
        code: "INVALID_PLACEMENT",
        message: "a value set in another scope reached the resolution",
    };
    expect([
        failure(() => editor.resolve(selection, [{ ...personal, scope: "user-bob" }], chain)),
        failure(() => editor.resolve(anonymous, [personal], chain)),
        failure(() =>
            editor.resolve(selection, [{ ...required, scope: "space-elsewhere" }], chain),
        ),
    ]).toEqual([
        misplaced,
        misplaced,
        {
            code: "INVALID_PLACEMENT",
            message: "a value from outside the selection's scope chain reached the resolution",
        },
    ]);
    expect(editor.resolve(anonymous, [], chain)).toEqual({ ...standard, selection: anonymous });

    // refuse disagreeing requirements and a selection of another scope than the setting's
    expect([
        failure(() =>
            editor.resolve(selection, [required, { ...required, value: "standard" }], chain),
        ),
        failure(() => editor.resolve(SettingSelection.parse({ scope: space }), [], chain)),
    ]).toEqual([
        { code: "CONFLICT", message: "required setting values disagree" },
        {
            code: "INVALID_PLACEMENT",
            message: "setting scope does not match the selected scope",
        },
    ]);
});

test("resolve each key of a merged setting from its own nearest placement, requirements fixing theirs", () => {
    // bind one command personally, another on the client, and require a third in the space
    const bindings = { ...personal, ...named(keybindings) };
    const mine = { ...bindings, value: { "note.archive": "mod+e", "note.pin": "mod+p" } };
    const laptop = { ...override, ...named(keybindings), value: { "note.search": "mod+k" } };
    const fixed = { ...required, ...named(keybindings), value: { "note.pin": null } };
    const resolved = keybindings.resolve(selection, [laptop, fixed, mine], chain);

    // keep every key from the placement deciding it, the client override leaving the personal keys
    expect(resolved).toEqual({
        setting: keybindings.reference,
        selection,
        value: { "note.archive": "mod+e", "note.pin": null, "note.search": "mod+k" },
        sources: [{ kind: "default", package: notes }, source(mine), source(laptop), source(fixed)],
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
            "note.search": { source: source(laptop), overridden: [], enforcement: "ordinary" },
        },
    });

    // refuse equally placed values disagreeing on a key
    expect(
        failure(() =>
            keybindings.resolve(
                selection,
                [mine, { ...mine, id: laptop.id, value: { "note.archive": "mod+r" } }],
                chain,
            ),
        ),
    ).toEqual({
        code: "CONFLICT",
        message: "multiple values of note.archive have the same precedence",
    });
});

test("compare required structured values independently of key order", () => {
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
        overrides: ["space", "installation", "client"],
        apply: "immediate",
    });
    const described = schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(JSON.stringify(describeSetting(editor))));
    expect(settingVocabulary(described)).toEqual({
        "editor.mode": { scope: "user" },
    });

    // read both settings back, validating values by their described schemas
    const catalog = await SettingCatalog.read(await release([editor, layout]));
    const read = catalog.get(layout.reference);
    const [first] = catalog.settings;
    if (first === undefined) {
        throw new TypeError("the catalog has no settings");
    }
    expect([
        catalog.settings.map(describeSetting),
        first.definition.schema.safeParse("vim").success,
        first.definition.schema.safeParse("emacs").success,
        read.definition.schema.safeParse({ width: 100, compact: true }).success,
        read.definition.schema.safeParse({ width: 0, compact: true }).success,
    ]).toEqual([[describeSetting(editor), describeSetting(layout)], true, false, true, false]);

    // refuse a setting the build does not declare
    expect(failure(() => catalog.get({ ...editor.reference, name: "theme" }))).toEqual({
        code: "UNDECLARED",
        message: `setting ${notes.id}/theme is not declared`,
    });
});

test("resolve space settings per installation and machine settings per machine", () => {
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

    // resolve a machine setting on its machine, and refuse one set on another machine
    const cache = defineSetting(
        {
            ...editor.definition,
            name: "cacheDirectory",
            scope: "machine",
            overrides: [],
            schema: schema.string().min(1),
            default: "/cache",
            apply: "restart",
        },
        { package: notes },
    );
    const machine = "machine-019f5530-8000-7000-8000-000000000015";
    const path: SettingValue = {
        ...personal,
        ...named(cache),
        scope: machine,
        value: "/Users/alice/cache",
    };
    const machineChain = [machine, account, Scope.universe.id];
    const onMachine = SettingSelection.parse({ scope: machine });
    expect(cache.resolve(onMachine, [path], machineChain)).toEqual({
        setting: cache.reference,
        selection: onMachine,
        value: "/Users/alice/cache",
        sources: [source(path)],
        overridden: [{ kind: "default", package: notes }],
        enforcement: "ordinary",
    });
    expect(
        failure(() =>
            cache.resolve(
                SettingSelection.parse({ scope: "machine-019f5530-8000-7000-8000-000000000016" }),
                [path],
                machineChain,
            ),
        ),
    ).toEqual({
        code: "INVALID_PLACEMENT",
        message: "a value set in another scope reached the resolution",
    });
});

test("refuse placements and writes a setting does not permit", () => {
    // refuse overrides the setting does not declare, and set values outside its scope
    const plain = defineSetting({ ...editor.definition, overrides: [] }, { package: notes });
    const unsupported = {
        code: "INVALID_PLACEMENT",
        message: "setting value uses an unsupported setting override",
    };
    expect([
        failure(() => plain.requirePlacement({ installation, clientId, mode: "set" }, "own")),
        failure(() => plain.resolve(selection, [override], chain)),
        failure(() => editor.requirePlacement({ mode: "set" }, "enclosing")),
        failure(() => editor.requirePlacement({ mode: "recommend" }, "own")),
        failure(() => editor.requirePlacement({ space, installation, mode: "set" }, "own")),
    ]).toEqual([
        unsupported,
        unsupported,
        {
            code: "INVALID_PLACEMENT",
            message: "setting value outside its declared scope must recommend or require",
        },
        {
            code: "INVALID_PLACEMENT",
            message: "setting value in its declared scope must be set",
        },
        {
            code: "INVALID_PLACEMENT",
            message: "setting value carries both an installation and its space",
        },
    ]);

    // accept a current write in its scope, and refuse one of a later release or with an invalid value
    const write = { mode: "set" as const, value: "vim", release: "2026.9.0" };
    expect([
        failure(() => editor.requireWrite(write, alice)),
        failure(() => editor.requireWrite({ ...write, release: "2026.10.0" }, alice)),
        failure(() => editor.requireWrite({ ...write, value: "emacs" }, alice)),
        failure(() => editor.requireWrite({ ...write, mode: "recommend" }, space)),
    ]).toEqual([
        "accepted",
        {
            code: "INVALID_VALUE",
            message: "setting value is at release 2026.10.0, its declaration at 2026.9.0",
        },
        { code: "INVALID_VALUE", message: "setting value does not match its declaration" },
        "accepted",
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

    // find the invalid value at its placement, and nothing where no value is placed
    const resolution = upgraded.resolve(selection, [personal], chain);
    expect([
        SettingResolution.observed(resolution, { scope: alice }),
        SettingResolution.observed(resolution, { scope: alice, clientId }),
    ]).toEqual([{ id: personal.id, revision: personal.revision }, null]);
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
    const current: SettingValue = { ...override, value: { keymap: "vim" } };
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

test("resolve a value for its consuming package, and rank recommendations by the nearness of their scope", () => {
    // apply a value for the consuming package in any space, and nowhere else
    const shared = defineSetting(
        {
            ...editor.definition,
            scope: "user",
            overrides: ["package", "space", "installation", "client"],
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
    expect(failure(() => shared.resolve(home, [nearer, sibling], chain))).toEqual({
        code: "CONFLICT",
        message: "multiple setting values have the same precedence",
    });
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
