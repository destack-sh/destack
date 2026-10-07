// @vitest-environment node
import { expect, test } from "@destack/test";
import type { Plugin } from "@destack/package/build";
import { originalPositionFor, TraceMap } from "@jridgewell/trace-mapping";
import { rolldown } from "rolldown";
import { iconExtension } from "./build.ts";

/** Return the icon plugin the extension transforms modules with. */
function iconPlugin(): Plugin {
    const [plugin] =
        iconExtension.transform?.({
            directory: "/package",
            runtime: "browser",
            server: false,
            options: {},
        }) ?? [];
    if (
        typeof plugin !== "object" ||
        plugin === null ||
        Array.isArray(plugin) ||
        plugin instanceof Promise
    ) {
        throw new TypeError("expected the icon plugin");
    }

    return plugin;
}

/** Call a plugin hook's handler outside a bundler, without a plugin context. */
function invoke(
    hook: Plugin["load"] | Plugin["transform"],
    parameters: readonly string[],
): unknown {
    // require an object hook
    if (hook === undefined || typeof hook === "function") {
        throw new TypeError("expected an object hook");
    }

    return Reflect.apply(hook.handler, undefined, parameters);
}

/** Build a module through the icon plugin, reading the refusal the bundler reports with its line and column. */
async function refusalOf(source: string, id: string): Promise<string> {
    // serve the module from memory and leave the icon package external
    const build = rolldown({
        input: id,
        external: [/^@destack\/icon/u],
        plugins: [
            {
                name: "source",
                resolveId: (name) => (name === id ? id : undefined),
                load: () => source,
            },
            iconPlugin(),
        ],
    });

    // read the plugin's message and where the bundler places it
    try {
        await (await build).generate();
    } catch (error) {
        const [refusal] = isBuildFailure(error) ? error.errors : [];

        return `${String(refusal?.loc?.line)}:${String(refusal?.loc?.column)} ${String(refusal?.message)}`;
    }
    throw new TypeError(`expected the build of ${id} to fail`);
}

/** A failed build's errors, each with its message and location. */
interface BuildFailure {
    /** The errors. */
    readonly errors: readonly {
        readonly message: string;
        readonly loc?: { readonly line: number; readonly column: number };
    }[];
}

/** Report whether a thrown value is a failed build carrying its errors. */
function isBuildFailure(error: unknown): error is BuildFailure {
    return error instanceof Error && "errors" in error && Array.isArray(error.errors);
}

test("pass each literal-named icon its bodies from a static import of its module", () => {
    // transform a module drawing two literal-named icons, one twice, and passed bodies
    const icon = iconPlugin();
    const transformed = invoke(icon.transform, [
        [
            'import { Icon as Glyph, type IconProperties } from "@destack/icon";',
            "",
            "/** The toolbar of a note, named é. */",
            "export const Toolbar = (properties: IconProperties) => (",
            "    <nav>",
            '        <Glyph name="trash" />',
            '        <Glyph name={"trash"} size={16} />',
            '        <Glyph weight="bold" name="heart-straight" label="Like" />',
            "        <Glyph icon={properties.icon} />",
            "    </nav>",
            ");",
            "",
        ].join("\n"),
        "/package/src/toolbar.tsx",
    ]);

    // require transformed code with a source map
    if (
        typeof transformed !== "object" ||
        transformed === null ||
        !("code" in transformed) ||
        !("map" in transformed) ||
        typeof transformed.map !== "string"
    ) {
        throw new TypeError("expected transformed code with a source map");
    }

    // import each icon's module once and pass it after the name, mapping the inserted attribute to the name
    const map = new TraceMap(transformed.map);
    expect({
        code: transformed.code,
        inserted: originalPositionFor(map, { line: 10, column: 51 }),
        following: originalPositionFor(map, { line: 10, column: 67 }),
    }).toEqual({
        inserted: { source: "/package/src/toolbar.tsx", line: 8, column: 49, name: null },
        following: { source: "/package/src/toolbar.tsx", line: 8, column: 51, name: null },
        code: [
            'import { Icon as Glyph, type IconProperties } from "@destack/icon";',
            'import __icon_0 from "@destack/icon/phosphor/trash";',
            'import __icon_1 from "@destack/icon/phosphor/heart-straight";',
            "",
            "/** The toolbar of a note, named é. */",
            "export const Toolbar = (properties: IconProperties) => (",
            "    <nav>",
            '        <Glyph name="trash" icon={__icon_0} />',
            '        <Glyph name={"trash"} icon={__icon_0} size={16} />',
            '        <Glyph weight="bold" name="heart-straight" icon={__icon_1} label="Like" />',
            "        <Glyph icon={properties.icon} />",
            "    </nav>",
            ");",
            "",
        ].join("\n"),
    });
});

test("refuse an icon with neither a literal name nor bodies, naming both alternatives", async () => {
    // transform a module drawing an icon whose name is computed
    const source = [
        'import { Icon, type IconName } from "@destack/icon";',
        "",
        "/** An icon chosen at run time. */",
        "export const Chosen = (properties: { name: IconName }) => <Icon name={properties.name} />;",
        "",
    ].join("\n");

    // report the element's line and column and both alternatives
    expect(await refusalOf(source, "/package/src/chosen.tsx")).toBe(
        "4:58 icon without a literal name or bodies: pass icon imported from @destack/icon/phosphor/<name>, or draw LazyIcon from @destack/icon/lazy",
    );
});

test("leave icons with passed bodies as written, with or without spread properties", () => {
    // transform a module whose icons take their bodies from properties
    const icon = iconPlugin();
    const transformed = invoke(icon.transform, [
        [
            'import * as icons from "@destack/icon";',
            'import { Icon, type IconProperties } from "@destack/icon";',
            "",
            "/** An icon drawn three ways. */",
            "export const Drawn = (properties: IconProperties) => [",
            "    <Icon {...properties} icon={properties.icon} />,",
            "    <Icon icon={properties.icon} />,",
            "    <icons.Icon icon={properties.icon} />,",
            "];",
            "",
        ].join("\n"),
        "/package/src/drawn.tsx",
    ]);

    // return no transformation, keeping the module as written
    expect(transformed).toBeUndefined();
});

test("refuse an icon with spread properties and no bodies, whose name the build cannot read", async () => {
    // transform a module drawing an icon whose properties may carry its name
    const source = [
        'import { Icon, type IconProperties } from "@destack/icon";',
        "",
        "/** An icon drawn from its properties. */",
        'export const Spread = (properties: IconProperties) => <Icon {...properties} name="trash" />;',
        "",
    ].join("\n");

    // report the element's line and column and both alternatives
    expect(await refusalOf(source, "/package/src/spread.tsx")).toBe(
        "4:54 icon with spread properties and no bodies: pass icon imported from @destack/icon/phosphor/<name>, or draw LazyIcon from @destack/icon/lazy",
    );
});

test("refuse an icon drawn through a namespace import without bodies", async () => {
    // transform a module drawing the icon component as a namespace member
    const source = [
        'import * as icons from "@destack/icon";',
        "",
        "/** A trash icon. */",
        'export const Trash = () => <icons.Icon name="trash" />;',
        "",
    ].join("\n");

    // report the element's line and column and the alternatives
    expect(await refusalOf(source, "/package/src/trash.tsx")).toBe(
        "4:27 icon drawn through a namespace import without bodies: import Icon by name, pass icon imported from @destack/icon/phosphor/<name>, or draw LazyIcon from @destack/icon/lazy",
    );
});
