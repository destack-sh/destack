import type { BrowserEnvironment } from "@destack/environment/browser";
import type { Package } from "@destack/package";
import { TelemetryScope } from "@destack/telemetry";
import { type Component, createComponent } from "solid-js";
import { type JSX, render } from "@solidjs/web";
import type { Localization } from "@destack/locale";
import { LocaleContext } from "./locale.ts";
import {
    EnvironmentContext,
    TelemetryContext,
    type ViewMount,
    ViewMountContext,
} from "./context.ts";

/** What a view's components render under, mounted in a page or prerendered as a static page. */
export interface ViewRootProperties {
    /** The package rendering, whose telemetry the components record through. */
    readonly package: Package;
    /** The environment the components run in. */
    readonly environment: BrowserEnvironment;
    /** The person's localization. */
    readonly localization: Localization;
    /** The mounted view, absent on a static page. */
    readonly mount?: ViewMount;
    /** The view's components. */
    readonly children: JSX.Element;
}

/** Render a view's components under its environment, telemetry, localization and mount. */
export function ViewRoot(properties: ViewRootProperties): JSX.Element {
    // provide the environment and the package's telemetry
    const telemetry = TelemetryScope.of(properties.package, properties.environment.telemetry);
    const localized = () =>
        createComponent(LocaleContext, {
            value: properties.localization,
            get children() {
                return properties.children;
            },
        });
    const { mount } = properties;

    return createComponent(EnvironmentContext, {
        value: properties.environment,
        get children() {
            return createComponent(TelemetryContext, {
                value: telemetry,
                get children() {
                    return mount === undefined
                        ? localized()
                        : createComponent(ViewMountContext, {
                              value: mount,
                              get children() {
                                  return localized();
                              },
                          });
                },
            });
        },
    });
}

/** Render a component into an element as a mounted view. */
export function renderView(
    component: Component,
    element: HTMLElement,
    root: Omit<ViewRootProperties, "children">,
): () => void {
    return render(
        () =>
            createComponent(ViewRoot, {
                ...root,
                get children() {
                    return createComponent(component, {});
                },
            }),
        element,
    );
}
