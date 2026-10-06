import type { JSX } from "@solidjs/web";
import { render } from "@solidjs/web";
import { createComponent } from "solid-js";
import { type Catalog, Localization, LocaleTag } from "@destack/locale";
import { LocaleContext } from "@destack/locale/solid";
import type { Example } from "@destack/package/declare";
import type { Environment } from "../scenario/interaction.ts";
import { View } from "../declare/view.ts";
import { ViewExample } from "./view.ts";
import { schema } from "@destack/schema";
import {
    Appearance,
    Contrast,
    DEFAULT_PREFERENCES,
    Density,
    Motion,
    Preset,
    TextSize,
    type Theme,
} from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";

/** The language an example renders in when its environment names none, the source language. */
const SOURCE_LOCALE = "en";

/** The theme settings an environment sets, each the person's default when absent. */
const ThemeSettings = schema.object({
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
    accent: Preset.nullable().exactOptional(),
});

/** What an example frame shows: the example, the environment it renders in and the properties a control changed. */
export interface ExampleFrame<Properties extends object = object> {
    /** The example. */
    readonly example: Example<Properties, JSX.Element>;
    /** The environment, the host's own when absent. */
    readonly environment?: Environment;
    /** The properties a control changed, the example's own when absent. */
    readonly properties?: Properties;
    /** The catalogs translating the messages of the example's package and its dependencies. */
    readonly catalogs?: readonly Catalog[];
    /** The theme of the example's package, Destack's own when absent. */
    readonly theme?: Theme;
}

/** Render an example into an element in one environment: its locale, theme settings, direction and width, returning how to unmount it. */
export function renderExample<Properties extends object>(
    element: HTMLElement,
    frame: ExampleFrame<Properties>,
): () => void {
    // read the environment's locale and theme settings
    const environment = frame.environment ?? {};
    const locale = LocaleTag.parse(environment.locale ?? SOURCE_LOCALE);
    const { appearance = "system", ...preferences } = ThemeSettings.parse(environment.theme ?? {});
    const theme = frame.theme ?? destackTheme;

    // frame the example in the locale and the direction components read, styled with the theme's variables and the width
    const direction =
        environment.direction === undefined ? {} : { direction: environment.direction };
    const localization = Localization.of(locale, frame.catalogs ?? [], direction);
    const container = document.createElement("div");
    container.dataset["slot"] = "example";
    container.lang = locale;
    container.dir = localization.direction;
    const style = theme.variables(appearance, { ...DEFAULT_PREFERENCES, ...preferences });
    for (const [name, value] of Object.entries(style)) {
        container.style.setProperty(name, value);
    }
    if (environment.width !== undefined) {
        container.style.width = `${environment.width}px`;
    }
    element.append(container);

    // render the example under the localization, which gives components the frame's direction, removing the frame on unmount
    const dispose = render(
        () =>
            createComponent(LocaleContext, {
                value: localization,
                get children() {
                    return renderedOf(frame);
                },
            }),
        container,
    );

    return () => {
        dispose();
        container.remove();
    };
}

/** Render what an example shows: a view over the objects its calls bring about, or the example's own rendering. */
function renderedOf<Properties extends object>(frame: ExampleFrame<Properties>): JSX.Element {
    const { example } = frame;
    if (example.of instanceof View) {
        return createComponent(ViewExample, { view: example.of, objects: example.objects });
    } else if (example.render === undefined) {
        throw new TypeError(`example ${example.name} renders nothing and shows no view`);
    }

    return example.render(frame.properties);
}
