import type { BrowserEnvironment } from "@destack/environment/browser";
import type { JSX } from "@solidjs/web";
import { render } from "@solidjs/web";
import { createComponent } from "solid-js";
import { ExamplePageContext, type IndexEntry } from "../page/example.ts";
import { type Catalog, Localization, LocaleTag, SOURCE_LOCALE } from "@destack/locale";
import { LocaleContext } from "../page/locale.ts";
import { ActionRegistry } from "../page/action.ts";
import { PageRoot } from "../page/root.ts";
import type { Example } from "@destack/package/declare";
import type { Globals } from "../scenario/interaction.ts";
import { View } from "../declare/view.ts";
import { ViewExample } from "./view.ts";
import { schema } from "@destack/schema";
import {
    Accent,
    Appearance,
    Contrast,
    DEFAULT_DISPLAY,
    Density,
    Motion,
    TextSize,
    type Theme,
    Typeface,
} from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import { ViewError } from "../error/error.ts";
import { Reference } from "@destack/style";

/** The paint of an example's frame, the theme's paint of a page's root in `theme.css`. */
const FRAME_PAINT: Readonly<Record<string, string>> = {
    "background-color": Reference.css("{color.background}"),
    color: Reference.css("{color.foreground}"),
    "font-family": Reference.css("{font.text}"),
    "font-size": Reference.css("{text.base.fontSize}"),
    "line-height": Reference.css("{text.base.lineHeight}"),
    "accent-color": Reference.css("{swatch.accentSolid}"),
};

/** The display the globals set, each choice the person's default when absent. */
export const GlobalsDisplay = schema.object({
    /** The appearance. */
    appearance: Appearance.exactOptional(),
    /** The text size. */
    textSize: TextSize.exactOptional(),
    /** The density, or null for the theme's own. */
    density: Density.nullable().exactOptional(),
    /** The interface typeface, or null for the theme's. */
    typeface: Typeface.nullable().exactOptional(),
    /** The contrast. */
    contrast: Contrast.exactOptional(),
    /** The motion. */
    motion: Motion.exactOptional(),
    /** The accent, or null for the theme's own. */
    accent: Accent.nullable().exactOptional(),
});

/** What an example frame shows. */
export interface ExampleFrame<Properties extends object = object> {
    /** The example. */
    readonly example: Example<Properties, JSX.Element>;
    /** The globals, the host's own when absent. */
    readonly globals?: Globals;
    /** The properties a control changed, the example's own when absent. */
    readonly properties?: Properties;
    /** The catalogs translating the messages of the example's package and its dependencies. */
    readonly catalogs?: readonly Catalog[];
    /** The examples and scenarios its page shows, none outside one. */
    readonly index?: readonly IndexEntry[];
    /** The theme of the example's package, Destack's own when absent. */
    readonly theme?: Theme;
}

/** Read the custom properties of an example's frame: the theme's variables, then its paint. */
export function frameStyle(
    globals: Globals,
    theme: Theme = destackTheme,
): Readonly<Record<string, string>> {
    const display = GlobalsDisplay.parse(globals.theme ?? {});

    return { ...theme.variables({ ...DEFAULT_DISPLAY, ...display }), ...FRAME_PAINT };
}

/** Render an example into an element in a browser environment. */
export function renderExample<Properties extends object>(
    element: HTMLElement,
    frame: ExampleFrame<Properties>,
    environment: BrowserEnvironment,
): () => void {
    // read the locale and theme settings the example is given
    const given = frame.globals ?? {};
    const locale = LocaleTag.parse(given.locale ?? SOURCE_LOCALE);

    // frame the example in the locale and the direction components read, styled with the theme's variables and the width
    const direction = given.direction === undefined ? {} : { direction: given.direction };
    const localization = Localization.of(locale, frame.catalogs ?? [], direction);

    // tell examples what their page shows and the languages its catalogs translate into
    const catalogs = frame.catalogs ?? [];
    const locales = [...new Set(catalogs.map((catalog) => catalog.locale))];
    const page = { index: frame.index ?? [], locales };
    const container = document.createElement("div");
    container.dataset["slot"] = "example";
    container.lang = locale;
    container.dir = localization.direction;
    for (const [name, value] of Object.entries(frameStyle(given, frame.theme))) {
        container.style.setProperty(name, value);
    }

    // size the frame to the width given
    if (given.width !== undefined) {
        container.style.width = `${given.width}px`;
    }

    // centre a centred example as one group, and give a padded one room, as Storybook's layouts do
    if (frame.example.layout === "centered") {
        container.style.display = "grid";
        container.style.placeItems = "center";
        container.style.placeContent = "center";
        container.style.minHeight = "100dvh";
        container.style.padding = Reference.css("{space.6}");
        container.style.boxSizing = "border-box";
    } else if (frame.example.layout === "padded") {
        container.style.padding = Reference.css("{space.6}");
        container.style.boxSizing = "border-box";
    }

    // give an app the window's height, which it scrolls inside
    if (frame.example.layout === "fullscreen") {
        container.style.height = "100dvh";
        container.style.overflow = "hidden";
        container.style.display = "grid";
        container.style.gridTemplateColumns = "minmax(0, 1fr)";
    }
    element.append(container);

    // run the keys of the actions the example handles until it unmounts
    const closing = new AbortController();
    const actions = new ActionRegistry();
    actions.listen(closing.signal);

    // render the example under the localization, which gives components the frame's direction, removing the frame on unmount
    const dispose = render(
        () =>
            createComponent(PageRoot, {
                environment,
                actions,
                get children() {
                    return createComponent(LocaleContext, {
                        value: localization,
                        get children() {
                            return createComponent(ExamplePageContext, {
                                value: page,
                                get children() {
                                    return renderFrame(frame);
                                },
                            });
                        },
                    });
                },
            }),
        container,
    );

    return () => {
        closing.abort();
        dispose();
        container.remove();
    };
}

/** Render what an example shows. */
function renderFrame<Properties extends object>(frame: ExampleFrame<Properties>): JSX.Element {
    const { example } = frame;
    if (example.of instanceof View) {
        return createComponent(ViewExample, { view: example.of, objects: example.objects });
    } else if (example.render === undefined) {
        throw ViewError.of(
            "EXAMPLE_REQUIRED",
            `render function or view of example '${example.name}' is required`,
        );
    }

    return example.render(frame.properties);
}
