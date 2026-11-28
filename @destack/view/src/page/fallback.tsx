import { Failure } from "@destack/error";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { ViewState } from "../declare/state.ts";
import type { ErrorFallbackProperties, ViewFallbacks } from "../declare/view.ts";
import type { JSX } from "../solid/component.ts";
import { useEnvironment } from "./context.ts";
import { useLocale } from "./locale.ts";

/** The styles of a view's fallbacks: a page of its own, its state centred. */
const styles = style.create({
    page: {
        display: "grid",
        placeItems: "center",
        minBlockSize: "100dvh",
        padding: space[6],
        boxSizing: "border-box",
    },
    state: {
        display: "grid",
        justifyItems: "center",
        gap: space[2],
        maxInlineSize: "32rem",
        textAlign: "center",
    },
    title: {
        margin: 0,
        fontSize: "1.25rem",
        fontWeight: 600,
    },
    description: {
        margin: 0,
        color: color.mutedForeground,
    },
});

/** Hold a view's place while its code or data is pending, empty as most loads are local. */
export function LoadingFallback(): JSX.Element {
    return <main aria-busy="true" {...style.attrs(styles.page)} />;
}

/** Show the state a failure left a view in, and the way out where there is one. */
export function ErrorFallback(properties: ErrorFallbackProperties): JSX.Element {
    // read the state the failure left, and how it reads
    const locale = useLocale();
    const environment = useEnvironment();
    const state = () => ViewState.of(properties.failure);
    const described = () => ViewState.describe(state());

    return (
        <main data-state={state()} {...style.attrs(styles.page)}>
            <div {...style.attrs(styles.state)}>
                <h1 {...style.attrs(styles.title)}>{locale.render(described().title)}</h1>
                <p {...style.attrs(styles.description)}>{locale.render(described().description)}</p>

                {/* Sign in by loading the page again, else retry after a failure that may pass */}
                {state() === "unauthorized" ? (
                    <button type="button" onClick={() => environment.navigation.reload()}>
                        {locale.render(t.context("action")`Sign in`)}
                    </button>
                ) : Failure.isTransient(properties.failure) ? (
                    <button type="button" onClick={() => properties.retry()}>
                        {locale.render(t.context("action")`Try again`)}
                    </button>
                ) : undefined}
            </div>
        </main>
    );
}

/** The runtime's parts, which a view draws where it declares none of its own. */
export const FALLBACKS: ViewFallbacks = { loading: LoadingFallback, error: ErrorFallback };
