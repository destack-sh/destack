import type { BrowserEnvironment } from "@destack/environment/browser";
import "../styles/styles.ts";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import type { ThemeDescription } from "@destack/theme";
import { type Component, createComponent, NoHydration } from "solid-js";
import type { JSX } from "@solidjs/web";
import type { View } from "../declare/view.ts";
import type { ViewMount } from "../page/context.ts";
import { ActionRegistry } from "../page/action.ts";
import { ViewRoot, type ViewRootProperties } from "../page/mount.ts";
import { ViewDocument } from "./document.tsx";
import { IslandContext, IslandPage } from "./island.tsx";

/** The meta tag naming where a page's serving origin takes telemetry, absent where none does. */
export const TELEMETRY_META = "destack-telemetry";

/** A view's page: its component inside its page shell, rendered on the server and then hydrated. */
export class ViewPage {
    /** The view the page shows. */
    readonly view: View;
    /** The view's root component. */
    readonly component: Component;
    /** The page shell wrapping the component. */
    readonly document: Component<{ readonly children?: JSX.Element }>;
    /** The actions the components handle. */
    readonly actions: ActionRegistry;

    /** Create a page of a view's loaded component and shell. */
    constructor(
        view: View,
        component: Component,
        document: Component<{ readonly children?: JSX.Element }>,
    ) {
        // keep the view and its loaded modules
        this.view = view;
        this.component = component;
        this.document = document;
        this.actions = new ActionRegistry({ keys: view.keys });
    }

    /** Open a view's page, in the view's own shell or the runtime's under the package's theme. */
    static async open(
        view: View,
        declared: { readonly theme?: ThemeDescription },
    ): Promise<ViewPage> {
        // load the view's root and its own shell
        const { FALLBACKS } = await import("../page/fallback.tsx");
        const component = await view.load(FALLBACKS);
        const { render } = view;
        const own =
            render.mode === "static" && render.document !== undefined
                ? (await render.document()).default
                : undefined;

        // fall back to the runtime's shell under the package's theme
        const document =
            own ??
            ((properties: { readonly children?: JSX.Element }) =>
                createComponent(ViewDocument, {
                    ...(declared.theme === undefined ? {} : { theme: declared.theme }),
                    get children() {
                        return properties.children;
                    },
                }));

        return new ViewPage(view, component, document);
    }

    /** Render the page: the component under its view root and mount, inside the page shell. */
    render(
        mount: ViewMount | null,
        root: Pick<ViewRootProperties, "actions" | "localization" | "environment">,
    ): JSX.Element {
        // render the component under its view root, and the mount a host gives a client view, inside the shell
        const { component, view } = this;
        const { actions, localization, environment } = root;
        const page = (): JSX.Element =>
            createComponent(this.document, {
                get children() {
                    return createComponent(ViewRoot, {
                        package: view.package,
                        environment,
                        actions,
                        localization,
                        ...(mount === null ? {} : { mount }),
                        get children() {
                            return createComponent(component, {});
                        },
                    });
                },
            });

        // hydrate the page whole, or render it without hydration but for its islands
        if (view.render.mode !== "static" || view.render.hydrate !== "islands") {
            return page();
        }

        return createComponent(NoHydration, {
            get children() {
                return createComponent(IslandContext, {
                    value: new IslandPage(view.package),
                    get children() {
                        return page();
                    },
                });
            },
        });
    }

    /** Read the view root a page first renders with: its own actions, in the source language. */
    root(
        environment: BrowserEnvironment,
    ): Pick<ViewRootProperties, "actions" | "localization" | "environment"> {
        return {
            actions: this.actions,
            localization: Localization.of(SOURCE_LOCALE, []),
            environment,
        };
    }
}
