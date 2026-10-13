import { type Component, createComponent } from "solid-js";
import { render } from "@solidjs/web";
import { type Catalog, Localization, SOURCE_LOCALE } from "@destack/locale";
import { LocaleContext } from "./locale.ts";
import { type ViewMount, ViewMountContext } from "./view.ts";

/** Render a component into an element as a mounted view, in the person's locale. */
export function renderView(
    component: Component,
    element: HTMLElement,
    mount: ViewMount,
    catalogs: readonly Catalog[],
): () => void {
    // render the component under the view, its clients and the person's localization
    return render(
        () =>
            createComponent(ViewMountContext, {
                value: mount,
                get children() {
                    return createComponent(LocaleContext, {
                        value: Localization.of(mount.context.locale ?? SOURCE_LOCALE, catalogs),
                        get children() {
                            return createComponent(component, {});
                        },
                    });
                },
            }),
        element,
    );
}
