import "@destack/view/styles";
import type { BrowserEnvironment, Navigation } from "@destack/environment/browser";
import { OriginPath } from "@destack/package";
import type { Example } from "@destack/package/declare";
import { type JsonValue, nth, schema } from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { ACCENT_PRESETS } from "@destack/color";
import { DEFAULT_DISPLAY, Radius, TEXT_SCALES, Theme, TYPEFACES } from "@destack/theme";
import { THEMES } from "@destack/theme/declare";
import { text } from "@destack/theme/text";
import { color, radius, size, space, stroke, weight, width } from "@destack/theme/tokens.stylex";
import { type JSX, render } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    For,
    Show,
    untrack,
} from "solid-js";
import type { ExampleLocation } from "../build/examples.ts";
import { Globals } from "../scenario/interaction.ts";
import { GlobalsDisplay, renderExample } from "./frame.ts";
import { faceRules } from "../document/font.tsx";
import { destackFonts } from "../document/typeface.ts";

/** The locales an example is checked in: the source language, one more, and one right to left. */
const LOCALES = ["en-US", "de-AT", "ar-EG"] as const;

/** The widths an example is checked at in CSS pixels: a phone's and a tablet's. */
const WIDTHS = ["390", "768"] as const;

/** The prefix of every custom property a theme sets. */
const THEME_PREFIX = "--destack-";

/** The step each arrow key moves the focus by among a picker's options. */
const ARROW_STEPS: Readonly<Record<string, number>> = { ArrowDown: 1, ArrowUp: -1 };

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
        gridTemplateColumns: "minmax(0, 1fr)",
    },
    framed: {
        display: "flex",
        flexDirection: "column",
    },
    bar: {
        position: "sticky",
        top: 0,
        zIndex: 1,
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        paddingBlock: space[2],
        paddingInline: space[4],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
        backgroundColor: color.background,
    },
    back: {
        marginInlineEnd: space[2],
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.foreground },
        },
        textDecoration: "none",
    },
    stage: {
        flexGrow: 1,
    },
    options: {
        position: "fixed",
        inset: "auto",
        margin: 0,
        marginBlockStart: space[1],
        maxHeight: "60dvh",
        overflowY: "auto",
        display: {
            default: "none",
            ":popover-open": "flex",
        },
        flexDirection: "column",
        minWidth: "10rem",
        padding: space[1],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.foreground,
    },
    option: {
        paddingBlock: space[1],
        paddingInline: space[2],
        borderWidth: 0,
        borderRadius: radius[2],
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
            ":focus-visible": color.muted,
        },
        color: "inherit",
        font: "inherit",
        textAlign: "start",
        cursor: "pointer",
        outlineStyle: "none",
    },
});

/** Render the examples of a package: one on its own when the URL names it, else the list. */
export function hostExamples(
    element: HTMLElement,
    locations: readonly ExampleLocation[],
    environment: BrowserEnvironment,
): () => void {
    // declare Destack's fonts, the faces a person may choose among them, as documents do
    const faces = element.ownerDocument.createElement("style");
    faces.textContent = destackFonts.map((font) => faceRules(font)).join("\n");
    element.ownerDocument.head.append(faces);
    element.className = style.attrs(text.sm, styles.page).class ?? "";

    // render the one example the path names, as `<module>/<name>` below the reserved path
    const url = new URL(environment.navigation.currentEntry.url);
    const path = url.pathname.slice(OriginPath.examples.length).replace(/^\//u, "");
    const chosen = locations.find((each) => pathOf(each) === path);
    const dispose =
        chosen === undefined
            ? render(() => {
                  // follow the URL's settings, and theme the page with the ones shown
                  const settings = new ExampleSettings(url, environment.navigation);
                  createEffect(settings.shown, (query) => paint(element, query));

                  return <ExampleList locations={locations} settings={settings} />;
              }, element)
            : hostExample(element, chosen, url, environment);

    return () => {
        dispose();
        faces.remove();
    };
}

/** Render one example on the whole page, laid out as it declares, under the URL's globals. */
function hostExample(
    element: HTMLElement,
    location: ExampleLocation,
    url: URL,
    environment: BrowserEnvironment,
): () => void {
    // put the settings above the stage the example renders on
    element.className = style.attrs(text.sm, styles.page, styles.framed).class ?? "";
    const bar = element.ownerDocument.createElement("div");
    const stage = element.ownerDocument.createElement("div");
    element.append(bar, stage);

    // render the example once loaded, again only as its locale or width changes
    let dispose: (() => void) | undefined;
    const disposeBar = render(() => {
        // follow the URL's settings, then load the example and render it for each locale and width
        const settings = new ExampleSettings(url, environment.navigation);
        const [example, setExample] =
            createSignal<Example<Record<string, JsonValue>, JSX.Element>>();
        void loadExample(location).then((loaded) => setExample(() => loaded));
        const renderKey = createMemo(() => {
            // key the render by the loaded example's locale and width alone
            const globals = settings.query().globals;

            return example() === undefined
                ? undefined
                : `${String(globals.locale)} ${String(globals.width)}`;
        });
        createEffect(renderKey, (key) => {
            // render again in place of the last render
            const loaded = untrack(example);
            if (key === undefined || loaded === undefined) {
                return;
            }
            dispose?.();
            const query = untrack(() => settings.query());
            stage.className = style.attrs(styles.stage, styles[loaded.layout]).class ?? "";
            dispose = renderExample(
                stage,
                { example: loaded, globals: query.globals, theme: themeOf(query) },
                environment,
            );
        });

        // theme the page and the example's frame with the settings shown
        createEffect(settings.shown, (query) => {
            const frame = stage.querySelector<HTMLElement>("[data-slot=example]");
            for (const target of frame === null ? [element] : [element, frame]) {
                paint(target, query);
            }
        });

        return (
            <nav aria-label="Example settings" {...style.attrs(text.xs, styles.bar)}>
                <a
                    href={`${OriginPath.examples}${settings.url().search}`}
                    {...style.attrs(styles.back)}
                >
                    ← Examples
                </a>
                <Settings settings={settings} />
            </nav>
        );
    }, bar);

    return () => {
        disposeBar();
        dispose?.();
    };
}

/** The settings of a page of examples: the ones in its URL, and the ones a person previews. */
class ExampleSettings {
    /** The URL with the chosen settings. */
    readonly url: Accessor<URL>;
    /** The chosen settings, read once per change of the URL. */
    readonly query: Accessor<Query>;
    /** The settings shown: a previewed one over the chosen ones. */
    readonly shown: Accessor<Query>;
    /** Replace the URL with the chosen settings. */
    readonly #setUrl: (url: URL) => void;
    /** Replace the previewed URL. */
    readonly #setPreview: (url: URL | undefined) => void;

    /** The page's history, whose URL has the chosen settings. */
    readonly #navigation: Navigation;

    /** Start from a page's URL. */
    constructor(url: URL, navigation: Navigation) {
        // start with the URL's settings and no preview
        this.#navigation = navigation;
        const [chosen, setChosen] = createSignal(url, { ownedWrite: true });
        const [preview, setPreview] = createSignal<URL | undefined>(undefined, {
            ownedWrite: true,
        });
        this.url = chosen;
        this.#setUrl = setChosen;
        this.#setPreview = setPreview;

        // read the chosen and shown settings once per change
        this.query = createMemo(() => queryOf(chosen()));
        this.shown = createMemo(() => queryOf(preview() ?? chosen()));
    }

    /** Show a setting's value without choosing it, the default for an empty value. */
    preview(name: string, value: string): void {
        this.#setPreview(urlWith(this.url(), name, value));
    }

    /** Show the chosen settings again. */
    revert(): void {
        this.#setPreview(undefined);
    }

    /** Choose a setting's value, writing it into the page's URL. */
    choose(name: string, value: string): void {
        // set the value, end the preview, and write the URL over the current entry
        const url = urlWith(this.url(), name, value);
        this.#setUrl(url);
        this.#setPreview(undefined);
        this.#navigation.navigate(url.href, { history: "replace" });
    }
}

/** Read a URL with a setting's value, leaving the setting out for an empty value. */
function urlWith(url: URL, name: string, value: string): URL {
    const next = new URL(url);
    if (value === "") {
        next.searchParams.delete(name);
    }
    // else set the value
    else {
        next.searchParams.set(name, value);
    }

    return next;
}

/** Set the theme's custom properties a query asks for, removing the theme's others there. */
export function paint(element: HTMLElement, query: Query): void {
    // read the display the query asks for, refusing settings outside their choices
    const display = { ...DEFAULT_DISPLAY, ...GlobalsDisplay.parse(query.globals.theme ?? {}) };
    const variables = themeOf(query).variables(display);

    // remove the theme's properties this paint leaves out, then set each of this paint's
    const names = Array.from({ length: element.style.length }, (_, index) =>
        element.style.item(index),
    );
    const stale = names.filter((name) => name.startsWith(THEME_PREFIX) && !(name in variables));
    for (const name of stale) {
        element.style.removeProperty(name);
    }
    for (const [name, value] of Object.entries(variables)) {
        element.style.setProperty(name, value);
    }
}

/** Render the list of a package's examples by module, each a link under the URL's settings. */
function ExampleList(properties: {
    /** The examples. */
    readonly locations: readonly ExampleLocation[];
    /** The page's settings. */
    readonly settings: ExampleSettings;
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
        `${OriginPath.examples}/${pathOf(each)}${properties.settings.url().search}`;

    return (
        <main {...style.attrs(styles.list)}>
            {/* Heading */}
            <h1 {...style.attrs(text["2xl"], styles.title)}>Examples</h1>
            <p {...style.attrs(styles.muted)}>
                {properties.locations.length} examples, each on its own page with these settings in
                its URL.
            </p>

            {/* Search, and the settings every example opens with */}
            <div {...style.attrs(styles.settings)}>
                <input
                    type="search"
                    {...style.attrs(styles.control)}
                    aria-label="Search examples"
                    placeholder="Search"
                    onInput={(event) => setSearch(event.currentTarget.value)}
                />
                <Settings settings={properties.settings} />
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

/** Render the settings every example opens with, applied at once and written into the URL. */
function Settings(properties: {
    /** The page's settings. */
    readonly settings: ExampleSettings;
}): JSX.Element {
    const settings = properties.settings;
    const theme = () => settings.query().globals.theme;

    return (
        <>
            <Picker
                settings={settings}
                name="theme"
                label="Theme"
                value={settings.query().theme}
                options={THEMES.map((each) => each.definition.name)}
            />
            <Picker
                settings={settings}
                name="appearance"
                label="Appearance"
                value={textOf(theme()?.["appearance"])}
                options={["system", "light", "dark"]}
            />
            <Picker
                settings={settings}
                name="accent"
                label="Accent"
                value={textOf(theme()?.["accent"])}
                options={ACCENT_PRESETS}
            />
            <Picker
                settings={settings}
                name="radius"
                label="Corners"
                value={settings.query().radius}
                options={Radius.options}
            />
            <Picker
                settings={settings}
                name="typeface"
                label="Font"
                value={textOf(theme()?.["typeface"])}
                options={Object.keys(TYPEFACES)}
            />
            <Picker
                settings={settings}
                name="textSize"
                label="Text size"
                value={textOf(theme()?.["textSize"])}
                options={Object.keys(TEXT_SCALES)}
            />
            <Picker
                settings={settings}
                name="density"
                label="Density"
                value={textOf(theme()?.["density"])}
                options={["compact", "regular", "spacious"]}
            />
            <Picker
                settings={settings}
                name="locale"
                label="Locale"
                value={settings.query().globals.locale}
                options={LOCALES}
            />
            <Picker
                settings={settings}
                name="width"
                label="Width"
                value={settings.query().globals.width?.toString()}
                options={WIDTHS}
            />
        </>
    );
}

/** Render a choice of one setting: a button opening its values, previewed as each is pointed at. */
function Picker(properties: {
    /** The page's settings. */
    readonly settings: ExampleSettings;
    /** The URL parameter it sets. */
    readonly name: string;
    /** The label, which stands for the default. */
    readonly label: string;
    /** The chosen value, the default when absent. */
    readonly value: string | undefined;
    /** The values to choose from. */
    readonly options: readonly string[];
}): JSX.Element {
    // anchor the list under its button, by the setting's name
    const id = `setting-${properties.name}`;
    const anchor = `--${id}`;
    let list: HTMLDivElement | undefined;
    const settings = properties.settings;

    // preview the option pointed at or focused, choose the one pressed, and step by the arrows
    const choose = (value: string) => {
        settings.choose(properties.name, value);
        list?.hidePopover();
    };
    const move = (event: KeyboardEvent) => {
        // move the focus to the option above or below the focused one
        const options = [...(list?.querySelectorAll<HTMLElement>("[role=option]") ?? [])];
        const index = options.findIndex((option) => option === event.target);
        const step = ARROW_STEPS[event.key];
        const next = step === undefined ? undefined : options[index + step];
        if (next !== undefined) {
            event.preventDefault();
            next.focus();
        }
    };

    return (
        <>
            <button
                type="button"
                popovertarget={id}
                aria-label={properties.label}
                style={`anchor-name: ${anchor}`}
                {...style.attrs(styles.control)}
            >
                {properties.value ?? properties.label}
            </button>
            <div
                id={id}
                ref={list}
                popover="auto"
                role="listbox"
                aria-label={properties.label}
                style={`position-anchor: ${anchor}; top: anchor(bottom); left: anchor(left)`}
                onToggle={(event: ToggleEvent) =>
                    event.newState === "closed"
                        ? settings.revert()
                        : list?.querySelector<HTMLElement>("[aria-selected=true]")?.focus()
                }
                onPointerLeave={() => settings.revert()}
                onKeyDown={move}
                {...style.attrs(styles.options)}
            >
                <For each={["", ...properties.options]}>
                    {(option) => (
                        <button
                            type="button"
                            role="option"
                            aria-selected={
                                (option || undefined) === properties.value ? "true" : "false"
                            }
                            onPointerEnter={() => settings.preview(properties.name, option)}
                            onFocus={() => settings.preview(properties.name, option)}
                            onClick={() => choose(option)}
                            {...style.attrs(styles.option)}
                        >
                            {option === "" ? properties.label : option}
                        </button>
                    )}
                </For>
            </div>
        </>
    );
}

/** Read what a URL's query asks for, refusing values outside their choices. */
export function queryOf(url: URL): Query {
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
            theme: entries(["appearance", "density", "textSize", "motion", "accent", "typeface"]),
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
