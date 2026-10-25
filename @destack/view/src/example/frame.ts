import type { BrowserEnvironment } from "@destack/environment/browser";
import type { JSX } from "@solidjs/web";
import { render } from "@solidjs/web";
import { createComponent } from "solid-js";
import { type Catalog, Localization, LocaleTag, SOURCE_LOCALE } from "@destack/locale";
import { LocaleContext } from "../page/locale.ts";
import { EnvironmentContext } from "../page/view.ts";
import type { Example } from "@destack/package/declare";
import type { Globals } from "../scenario/interaction.ts";
import { View } from "../declare/view.ts";
import { ViewExample } from "./view.ts";
import { schema } from "@destack/schema";
import {
    Appearance,
    Contrast,
    DEFAULT_DISPLAY,
    Density,
    Motion,
    Seed,
    TextSize,
    type Theme,
} from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";
import { ViewError } from "../error/error.ts";

/** The display the globals set, each choice the person's default when absent. */
const GlobalsDisplay = schema.object({
    /** The appearance. */
    appearance: Appearance.exactOptional(),
    /** The text size. */
    textSize: TextSize.exactOptional(),
    /** The density, or null for the theme's own. */
    density: Density.nullable().exactOptional(),
    /** The contrast. */
    contrast: Contrast.exactOptional(),
    /** The motion. */
    motion: Motion.exactOptional(),
    /** The accent, or null for the theme's own. */
    accent: Seed.nullable().exactOptional(),
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
    /** The theme of the example's package, Destack's own when absent. */
    readonly theme?: Theme;
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
    const display = GlobalsDisplay.parse(given.theme ?? {});
    const theme = frame.theme ?? destackTheme;

    // frame the example in the locale and the direction components read, styled with the theme's variables and the width
    const direction = given.direction === undefined ? {} : { direction: given.direction };
    const localization = Localization.of(locale, frame.catalogs ?? [], direction);
    const container = document.createElement("div");
    container.dataset["slot"] = "example";
    container.lang = locale;
    container.dir = localization.direction;
    const style = theme.variables({ ...DEFAULT_DISPLAY, ...display });
    for (const [name, value] of Object.entries(style)) {
        container.style.setProperty(name, value);
    }
    if (given.width !== undefined) {
        container.style.width = `${given.width}px`;
    }
    element.append(container);

    // render the example under the localization, which gives components the frame's direction, removing the frame on unmount
    const dispose = render(
        () =>
            createComponent(EnvironmentContext, {
                value: environment,
                get children() {
                    return createComponent(LocaleContext, {
                        value: localization,
                        get children() {
                            return renderFrame(frame);
                        },
                    });
                },
            }),
        container,
    );

    return () => {
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
            "EXAMPLE_EMPTY",
            `example ${example.name} has neither a render function nor a view`,
        );
    }

    return example.render(frame.properties);
}
