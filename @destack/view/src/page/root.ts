import type { BrowserEnvironment } from "@destack/environment/browser";
import { createComponent } from "solid-js";
import type { JSX } from "@solidjs/web";
import type { ActionRegistry } from "./action.ts";
import { ActionRegistryContext, EnvironmentContext } from "./context.ts";

/** What every component renders under, in a view, on a static page, in an example or in a test. */
export interface PageRootProperties {
    /** The environment the components run in. */
    readonly environment: BrowserEnvironment;
    /** The actions the components handle. */
    readonly actions: ActionRegistry;
    /** The components. */
    readonly children: JSX.Element;
}

/** Render components under their environment and the actions they handle. */
export function PageRoot(properties: PageRootProperties): JSX.Element {
    return createComponent(EnvironmentContext, {
        value: properties.environment,
        get children() {
            return createComponent(ActionRegistryContext, {
                value: properties.actions,
                get children() {
                    return properties.children;
                },
            });
        },
    });
}
