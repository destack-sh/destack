import "@destack/view/styles";
import type { BrowserEnvironment } from "@destack/environment/browser";
import { OriginPath } from "@destack/package";
import type { Example } from "@destack/package/declare";
import { type JsonValue, nth, schema } from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { DEFAULT_DISPLAY, Radius, Theme } from "@destack/theme";
import { THEMES } from "@destack/theme/declare";
import { text } from "@destack/theme/text";
import { color, radius, size, space, stroke, weight, width } from "@destack/theme/tokens.stylex";
import { type JSX, render } from "@solidjs/web";
import { createSignal, For, Show } from "solid-js";
import type { ExampleLocation } from "../build/examples.ts";
import { Globals } from "../scenario/interaction.ts";
import { renderExample } from "./frame.ts";

/** The locales an example is checked in: the source language, one more, and one right to left. */
const LOCALES = ["en-US", "de-AT", "ar-EG"] as const;

/** The widths an example is checked at in CSS pixels: a phone's and a tablet's. */
const WIDTHS = ["390", "768"] as const;

/** The widest the list of examples runs, a laptop's content width. */
const LIST_WIDTH = "72rem";

/** What a page of examples shows, as its URL's query sets it. */
const Query = schema.object({
    /** The theme by name, Destack's brand when absent. */
    theme: schema.string().min(1).exactOptional(),
    /** The corners, the theme's when absent. */
    radius: Radius.exactOptional(),
    /** The globals the example renders with. */
    globals: Globals,
});
/** What a page of examples shows. */
type Query = schema.Infer<typeof Query>;

/** The styles of the pages of examples. */
const styles = style.create({
    page: {
        minHeight: "100dvh",
        margin: 0,
        backgroundColor: color.background,
        color: color.foreground,
    },
    list: {
        maxWidth: LIST_WIDTH,
        marginInline: "auto",
        paddingBlock: space[8],
        paddingInline: space[6],
    },
    title: {
        margin: 0,
        fontWeight: weight.medium,
    },
    muted: {
        marginBlock: space[1],
        color: color.mutedForeground,
    },
    settings: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[2],
        marginBlock: space[6],
    },
    form: {
        display: "contents",
    },
    control: {
        height: size[2],
        paddingInline: space[3],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.input,
        borderRadius: radius[3],
        backgroundColor: color.background,
        color: color.foreground,
        fontFamily: "inherit",
        fontSize: "inherit",
    },
    groups: {
        columnWidth: width.tile,
        columnGap: space[8],
    },
    group: {
        breakInside: "avoid",
        marginBlockEnd: space[6],
    },
    heading: {
        margin: 0,
        marginBlockEnd: space[1],
        fontWeight: weight.medium,
    },
    entries: {
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
    link: {
        display: "block",
        overflow: "hidden",
        paddingBlock: space[1],
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.foreground },
        },
        textDecoration: "none",
    },
    centered: {
        display: "grid",
        placeItems: "center",
        padding: space[6],
    },
    padded: {
        padding: space[6],
    },
    fullscreen: {
        display: "grid",
    },
});

/** Render the examples of a package: one on its own when the URL names it, else the list. */
export function hostExamples(
    element: HTMLElement,
    locations: readonly ExampleLocation[],
    environment: BrowserEnvironment,
): () => void {
    // read what the URL asks for, and theme the page with it
    const url = new URL(environment.navigation.currentEntry.url);
    const query = queryOf(url);
    const theme = themeOf(query);
    const variables = theme.variables({ ...DEFAULT_DISPLAY, ...query.globals.theme });
    for (const [name, value] of Object.entries(variables)) {
        element.style.setProperty(name, value);
    }
    element.className = style.attrs(text.sm, styles.page).class ?? "";

    // render the one example the path names, as `<module>/<name>` below the reserved path
    const path = url.pathname.slice(OriginPath.examples.length).replace(/^\//u, "");
    const chosen = locations.find((each) => pathOf(each) === path);
    if (chosen !== undefined) {
        return hostExample(element, chosen, query, theme, environment);
    }

    return render(() => <ExampleList locations={locations} query={query} url={url} />, element);
}

/** Render one example on the whole page, laid out as it declares, under the URL's globals. */
function hostExample(
    element: HTMLElement,
    location: ExampleLocation,
    query: Query,
    theme: Theme,
    environment: BrowserEnvironment,
): () => void {
    // load the example
    let dispose: (() => void) | undefined;
    void loadExample(location).then((example) => {
        // lay the page out as the example declares, and render it
        element.className = style.attrs(styles.page, styles[example.layout]).class ?? "";
        dispose = renderExample(element, { example, globals: query.globals, theme }, environment);

        // give a centred example's elements their own widths
        const container = element.querySelector(":scope > [data-slot=example]");
        if (container instanceof HTMLElement && example.layout === "centered") {
            container.style.display = "contents";
        }
    });

    return () => dispose?.();
}

/** Render the list of a package's examples by module, each a link under the URL's settings. */
function ExampleList(properties: {
    /** The examples. */
    readonly locations: readonly ExampleLocation[];
    /** What the URL asks for. */
    readonly query: Query;
    /** The page's URL. */
    readonly url: URL;
}): JSX.Element {
    // group the examples whose module and export match the search by module
    const [search, setSearch] = createSignal("");
    const groups = () => {
        const wanted = search().toLowerCase();
        const matching = properties.locations.filter((each) => pathOf(each).includes(wanted));

        return Object.entries(Object.groupBy(matching, (each) => groupOf(each.module)));
    };

    // link each example with the page's settings
    const linkOf = (each: ExampleLocation): string =>
        `${OriginPath.examples}/${pathOf(each)}${properties.url.search}`;
    const { query } = properties;

    return (
        <main {...style.attrs(styles.list)}>
            {/* Heading */}
            <h1 {...style.attrs(text["2xl"], styles.title)}>Examples</h1>
            <p {...style.attrs(styles.muted)}>
                {properties.locations.length} examples, each on its own page with these settings in
                its URL.
            </p>

            {/* Search, and the settings every example opens with, applied by reloading */}
            <div {...style.attrs(styles.settings)}>
                <input
                    type="search"
                    {...style.attrs(styles.control)}
                    aria-label="Search examples"
                    placeholder="Search"
                    onInput={(event) => setSearch(event.currentTarget.value)}
                />
                <form
                    method="get"
                    onChange={(event) => event.currentTarget.requestSubmit()}
                    {...style.attrs(styles.form)}
                >
                    <Choice
                        name="theme"
                        label="Theme"
                        value={query.theme}
                        options={THEMES.map((each) => each.definition.name)}
                    />
                    <Choice
                        name="radius"
                        label="Corners"
                        value={query.radius}
                        options={Radius.options}
                    />
                    <Choice
                        name="appearance"
                        label="Appearance"
                        value={textOf(query.globals.theme?.["appearance"])}
                        options={["light", "dark"]}
                    />
                    <Choice
                        name="density"
                        label="Density"
                        value={textOf(query.globals.theme?.["density"])}
                        options={["compact", "regular", "spacious"]}
                    />
                    <Choice
                        name="locale"
                        label="Locale"
                        value={query.globals.locale}
                        options={LOCALES}
                    />
                    <Choice
                        name="width"
                        label="Width"
                        value={query.globals.width?.toString()}
                        options={WIDTHS}
                    />
                </form>
            </div>

            {/* Examples by module */}
            <div {...style.attrs(styles.groups)}>
                <For each={groups()}>
                    {([group, entries]) => (
                        <section {...style.attrs(styles.group)}>
                            <h2 {...style.attrs(text.sm, styles.heading)}>{group}</h2>
                            <ul {...style.attrs(styles.entries)}>
                                <For each={entries ?? []}>
                                    {(each) => (
                                        <li>
                                            <a
                                                href={linkOf(each)}
                                                title={each.export}
                                                {...style.attrs(styles.link)}
                                            >
                                                {each.name}
                                            </a>
                                        </li>
                                    )}
                                </For>
                            </ul>
                        </section>
                    )}
                </For>
            </div>
            <Show when={groups().length === 0}>
                <p {...style.attrs(styles.muted)}>No example matches.</p>
            </Show>
        </main>
    );
}

/** Render a choice of one URL setting, its label standing for the default. */
function Choice(properties: {
    /** The URL parameter it sets. */
    readonly name: string;
    /** The label. */
    readonly label: string;
    /** The chosen value, the default when absent. */
    readonly value: string | undefined;
    /** The values to choose from. */
    readonly options: readonly string[];
}): JSX.Element {
    return (
        <select
            name={properties.name}
            aria-label={properties.label}
            {...style.attrs(styles.control)}
        >
            <option value="" selected={properties.value === undefined}>
                {properties.label}
            </option>
            <For each={properties.options}>
                {(option) => (
                    <option value={option} selected={option === properties.value}>
                        {option}
                    </option>
                )}
            </For>
        </select>
    );
}

/** Read what a URL's query asks for, refusing values outside their choices. */
function queryOf(url: URL): Query {
    // read each parameter, taking the empty ones a form sends for its defaults as absent
    const read = (name: string): string | undefined => {
        const value = url.searchParams.get(name);

        return value === null || value === "" ? undefined : value;
    };
    const entries = (names: readonly string[]) =>
        Object.fromEntries(
            names.flatMap((name) => {
                const value = read(name);

                return value === undefined ? [] : [[name, value]];
            }),
        );
    const pixels = read("width");

    return Query.parse({
        ...entries(["theme", "radius"]),
        globals: {
            theme: entries(["appearance", "density"]),
            ...entries(["locale"]),
            ...(pixels === undefined ? {} : { width: Number(pixels) }),
        },
    });
}

/** Read the theme a query asks for, with its corners. */
function themeOf(query: Query): Theme {
    const theme = THEMES.find((each) => each.definition.name === query.theme) ?? nth(THEMES, 0);

    return query.radius === undefined
        ? theme
        : new Theme(theme.package, { ...theme.definition, radius: query.radius });
}

/** Read a JSON value as text, absent unless it is a string. */
function textOf(value: JsonValue | undefined): string | undefined {
    return typeof value === "string" ? value : undefined;
}

/** Read an example's key: its module and export. */
function keyOf(location: ExampleLocation): string {
    return `${location.module}#${location.export}`;
}

/** Read an example's path below the reserved path: its module's directory and its name. */
function pathOf(location: ExampleLocation): string {
    return `${groupOf(location.module)}/${location.name}`;
}

/** Read the directory of an example's module. */
function groupOf(module: string): string {
    return module.split("/").at(-2) ?? module;
}

/** Load an example from its module. */
async function loadExample(
    location: ExampleLocation,
): Promise<Example<Record<string, JsonValue>, JSX.Element>> {
    // import the module and read its export
    const loaded: unknown = await import(/* @vite-ignore */ location.module);
    const exports = schema.record(schema.string(), schema.unknown()).parse(loaded);
    const example = exports[location.export];
    if (!isExample(example)) {
        throw new TypeError(
            `example '${keyOf(location)}' is invalid: its export is not an example`,
        );
    }

    return example;
}

/** Report whether a value is an example a host renders. */
function isExample(value: unknown): value is Example<Record<string, JsonValue>, JSX.Element> {
    return (
        typeof value === "object" &&
        value !== null &&
        "description" in value &&
        "of" in value &&
        "render" in value
    );
}
