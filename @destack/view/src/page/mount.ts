import { type Component, createComponent } from "solid-js";
import { render } from "@solidjs/web";
import { type Catalog, Localization, SOURCE_LOCALE } from "@destack/locale";
import { LocaleContext } from "./locale.ts";
import type { ObjectClient } from "@destack/object/client";
import type { ViewContext } from "../declare/context.ts";
import { type ViewMount, ViewMountContext } from "./view.ts";

/** Render a component into an element as a mounted view over its scopes' clients and its platform services in the person's locale, returning how to unmount it. */
export function renderView(
    component: Component,
    element: HTMLElement,
    context: ViewContext,
    clients: Readonly<Record<string, ObjectClient>>,
    catalogs: readonly Catalog[],
    services: ViewMount["services"],
): () => void {
    // render the component under the view, its clients and the person's localization
    return render(
        () =>
            createComponent(ViewMountContext, {
                value: { context, clients: new Map(Object.entries(clients)), services },
                get children() {
                    return createComponent(LocaleContext, {
                        value: Localization.of(context.locale ?? SOURCE_LOCALE, catalogs),
                        get children() {
                            return createComponent(component, {});
                        },
                    });
                },
            }),
        element,
    );
}
