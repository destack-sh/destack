import "@destack/view/styles";
import type { BrowserEnvironment } from "@destack/environment/browser";
import type { Example } from "@destack/package/declare";
import { nth, schema } from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { DEFAULT_DISPLAY } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import { color, font, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { type JSX, render } from "@solidjs/web";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import type { Globals } from "../scenario/interaction.ts";
import { renderExample } from "./frame.ts";

/** The widths a reviewer checks an example at, in CSS pixels. */
const WIDTHS = { phone: 390, tablet: 768 } as const;

/** The locales a reviewer checks an example in. */
const LOCALES = ["en-US", "de-AT", "ar-EG"] as const;

/** The appearances a reviewer checks an example in. */
const APPEARANCES = ["light", "dark"] as const;

/** The module and export of an example. */
export interface ExampleLocation {
    /** The URL path of the module declaring it, such as /src/board/board.example.tsx. */
    readonly module: string;
    /** The export of the module. */
    readonly export: string;
}

/** One example a package declares, loaded from its module. */
interface ExampleEntry extends ExampleLocation {
    /** The example. */
    readonly example: Example<object, JSX.Element>;
}

/** The choices a reviewer makes. */
interface Choice {
    /** The chosen example, as `module#export`. */
    readonly example: string | undefined;
    /** The appearance. */
    readonly appearance: (typeof APPEARANCES)[number];
    /** The locale. */
    readonly locale: (typeof LOCALES)[number];
    /** The width, or the full page. */
    readonly width: keyof typeof WIDTHS | "full";
}

/** The styles of the examples page. */
const styles = style.create({
    page: {
        display: "grid",
        gridTemplateColumns: "16rem minmax(0, 1fr)",
        minHeight: "100vh",
        backgroundColor: color.background,
        color: color.foreground,
        fontFamily: font.text,
        fontSize: "14px",
    },
    sidebar: {
        position: "sticky",
        top: 0,
        height: "100vh",
        overflowY: "auto",
        padding: space[3],
        borderInlineEndWidth: stroke.border,
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
    },
    group: {
        marginBlockStart: space[3],
        marginBlockEnd: space[1],
        marginInline: 0,
        color: color.mutedForeground,
        fontSize: "12px",
        textTransform: "uppercase",
        letterSpacing: "0.04em",
    },
    link: {
        display: "block",
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        color: "inherit",
        textDecoration: "none",
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.accent },
        },
    },
    current: {
        backgroundColor: color.accent,
        fontWeight: weight.semibold,
    },
    main: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        padding: space[4],
        minWidth: 0,
    },
    bar: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
    title: {
        flexGrow: 1,
        margin: 0,
        fontSize: "16px",
    },
    description: {
        margin: 0,
        color: color.mutedForeground,
    },
    failed: {
        color: color.destructive,
    },
    frame: {
        width: "100%",
        minWidth: 0,
        marginInline: "auto",
    },
    phone: { maxWidth: WIDTHS.phone },
    tablet: { maxWidth: WIDTHS.tablet },
});

/** Render the examples of a package. */
export function hostExamples(
    element: HTMLElement,
    locations: readonly ExampleLocation[],
    environment: BrowserEnvironment,
): () => void {
    // load every example from its module
    const [entries, setEntries] = createSignal<readonly ExampleEntry[]>([]);
    const [failures, setFailures] = createSignal<readonly string[]>([]);
    void Promise.allSettled(locations.map((location) => loadExample(location))).then((settled) => {
        setEntries(
            settled.flatMap((result) => (result.status === "fulfilled" ? [result.value] : [])),
        );
        setFailures(
            settled.flatMap((result, index) =>
                result.status === "rejected"
                    ? [`${keyOf(nth(locations, index))}: ${String(result.reason)}`]
                    : [],
            ),
        );
    });

    return render(
        () => <ExamplesPage entries={entries()} failures={failures()} environment={environment} />,
        element,
    );
}

/** Render the examples page. */
function ExamplesPage(properties: {
    /** The examples. */
    readonly entries: readonly ExampleEntry[];
    /** The example modules that failed to load, with their errors. */
    readonly failures: readonly string[];
    /** The browser environment of the examples. */
    readonly environment: BrowserEnvironment;
}): JSX.Element {
    // keep the reviewer's choices in the URL
    const navigation = properties.environment.navigation;
    const [choice, setChoice] = createSignal(readChoice(new URL(navigation.currentEntry.url)));
    createEffect(choice, (current) => {
        navigation.navigate(`?${writeChoice(current).toString()}`, { history: "replace" });
    });
    const change = (next: Partial<Choice>) => setChoice({ ...choice(), ...next });
    const chosen = () =>
        properties.entries.find((entry) => keyOf(entry) === choice().example) ??
        properties.entries[0];

    // group the examples by the directory of their module
    const groups = () =>
        Object.entries(Object.groupBy(properties.entries, (entry) => groupOf(entry.module))).map(
            ([name, entries]) => ({ name, entries: entries ?? [] }),
        );

    // render the chosen example into the frame
    let frame: HTMLDivElement | undefined;
    createEffect(
        () => [chosen(), choice()] as const,
        ([entry, current]) => {
            // draw the example, removing the last one
            const target = frame;
            if (entry === undefined || target === undefined) {
                return undefined;
            }
            target.replaceChildren();

            return renderExample(
                target,
                { example: entry.example, globals: globalsOf(current) },
                properties.environment,
            );
        },
    );
    onCleanup(() => frame?.replaceChildren());

    // theme the page in the chosen appearance
    let page: HTMLDivElement | undefined;
    createEffect(
        () => choice().appearance,
        (appearance) => {
            const variables = destackTheme.variables({ ...DEFAULT_DISPLAY, appearance });
            for (const [name, value] of Object.entries(variables)) {
                page?.style.setProperty(name, value);
            }
        },
    );

    return (
        <div ref={(created) => (page = created)} {...style.attrs(styles.page)}>
            {/* Examples by module, after the modules that failed to load */}
            <nav aria-label="Examples" {...style.attrs(styles.sidebar)}>
                <For each={properties.failures}>
                    {(failure) => <p {...style.attrs(styles.failed)}>{failure}</p>}
                </For>
                <For each={groups()}>
                    {(group) => (
                        <>
                            <h2 {...style.attrs(styles.group)}>{group.name}</h2>
                            <For each={group.entries}>
                                {(entry) => {
                                    const isCurrent = (): boolean => chosen() === entry;

                                    return (
                                        <a
                                            href={`?example=${encodeURIComponent(keyOf(entry))}`}
                                            aria-current={isCurrent() ? "page" : undefined}
                                            onClick={(event) => {
                                                event.preventDefault();
                                                change({ example: keyOf(entry) });
                                            }}
                                            {...style.attrs(
                                                styles.link,
                                                isCurrent() && styles.current,
                                            )}
                                        >
                                            {entry.example.name}
                                        </a>
                                    );
                                }}
                            </For>
                        </>
                    )}
                </For>
            </nav>

            {/* The chosen example and its controls */}
            <main {...style.attrs(styles.main)}>
                <div {...style.attrs(styles.bar)}>
                    <h1 {...style.attrs(styles.title)}>{titleOf(chosen())}</h1>
                    <Picker
                        label="Appearance"
                        value={choice().appearance}
                        options={APPEARANCES}
                        onChange={(appearance) => change({ appearance })}
                    />
                    <Picker
                        label="Locale"
                        value={choice().locale}
                        options={LOCALES}
                        onChange={(locale) => change({ locale })}
                    />
                    <Picker
                        label="Width"
                        value={choice().width}
                        options={["full", "tablet", "phone"] as const}
                        onChange={(width) => change({ width })}
                    />
                </div>
                <p {...style.attrs(styles.description)}>{chosen()?.example.description}</p>
                <div
                    ref={(created) => (frame = created)}
                    data-slot="example-frame"
                    {...style.attrs(
                        styles.frame,
                        choice().width === "phone" && styles.phone,
                        choice().width === "tablet" && styles.tablet,
                    )}
                />
            </main>
        </div>
    );
}

/** Render a labelled choice among a few options. */
function Picker<Option extends string>(properties: {
    /** The label. */
    readonly label: string;
    /** The chosen option. */
    readonly value: Option;
    /** The options. */
    readonly options: readonly Option[];
    /** Handle another option being chosen. */
    readonly onChange: (value: Option) => void;
}): JSX.Element {
    return (
        <label>
            {properties.label}{" "}
            <select
                value={properties.value}
                onChange={(event) => {
                    const chosen = properties.options.find(
                        (option) => option === event.currentTarget.value,
                    );
                    if (chosen !== undefined) {
                        properties.onChange(chosen);
                    }
                }}
            >
                <For each={properties.options}>
                    {(option) => <option value={option}>{option}</option>}
                </For>
            </select>
        </label>
    );
}

/** Read the reviewer's choices from a URL. */
function readChoice(url: URL): Choice {
    const search = url.searchParams;
    const width = search.get("width");

    return {
        example: search.get("example") ?? undefined,
        appearance: search.get("appearance") === "dark" ? "dark" : "light",
        locale: LOCALES.find((each) => each === search.get("locale")) ?? LOCALES[0],
        width: width === "phone" || width === "tablet" ? width : "full",
    };
}

/** Write the reviewer's choices as URL parameters. */
function writeChoice(choice: Choice): URLSearchParams {
    return new URLSearchParams({
        ...(choice.example === undefined ? {} : { example: choice.example }),
        appearance: choice.appearance,
        locale: choice.locale,
        width: choice.width,
    });
}

/** Read the globals an example renders with from the reviewer's choices. */
function globalsOf(choice: Choice): Globals {
    return {
        theme: { appearance: choice.appearance },
        locale: choice.locale,
        ...(choice.width === "full" ? {} : { width: WIDTHS[choice.width] }),
    };
}

/** Read an example's key: its module and export. */
function keyOf(entry: ExampleLocation): string {
    return `${entry.module}#${entry.export}`;
}

/** Read the title of the chosen example. */
function titleOf(entry: ExampleEntry | undefined): string {
    return entry === undefined ? "No examples" : `${groupOf(entry.module)} · ${entry.example.name}`;
}

/** Read the directory of an example's module. */
function groupOf(module: string): string {
    return module.split("/").at(-2) ?? module;
}

/** Load an example from its module. */
async function loadExample(location: ExampleLocation): Promise<ExampleEntry> {
    // import the module and read its export
    const loaded: unknown = await import(/* @vite-ignore */ location.module);
    const exports = schema.record(schema.string(), schema.unknown()).parse(loaded);
    const example = exports[location.export];
    if (!isExample(example)) {
        throw new TypeError(`example '${keyOf(location)}' is invalid: its export holds no example`);
    }

    return { ...location, example };
}

/** Report whether a value is an example a host renders. */
function isExample(value: unknown): value is Example<object, JSX.Element> {
    return (
        typeof value === "object" &&
        value !== null &&
        "description" in value &&
        "of" in value &&
        "render" in value
    );
}
