import type { BrowserEnvironment } from "@destack/environment/browser";
import { type Component, createComponent } from "solid-js";
import { render } from "@solidjs/web";
import type { Localization } from "@destack/locale";
import { LocaleContext } from "./locale.ts";
import { EnvironmentContext, type ViewMount, ViewMountContext } from "./view.ts";

/** Render a component into an element as a mounted view, in a locale and an environment. */
export function renderView(
    component: Component,
    element: HTMLElement,
    mount: ViewMount,
    localization: Localization,
    environment: BrowserEnvironment,
): () => void {
    // render the component under the environment, the view, its clients and the person's localization
    return render(
        () =>
            createComponent(EnvironmentContext, {
                value: environment,
                get children() {
                    return createComponent(ViewMountContext, {
                        value: mount,
                        get children() {
                            return createComponent(LocaleContext, {
                                value: localization,
                                get children() {
                                    return createComponent(component, {});
                                },
                            });
                        },
                    });
                },
            }),
        element,
    );
}
