import type { IconBodies } from "@destack/icon";
import lockIcon from "@destack/icon/phosphor/lock-simple";
import signInIcon from "@destack/icon/phosphor/sign-in";
import signpostIcon from "@destack/icon/phosphor/signpost";
import warningIcon from "@destack/icon/phosphor/warning-circle";
import offlineIcon from "@destack/icon/phosphor/wifi-slash";
import { argument, type Message, t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import { type JSX, onCleanup, Show, useEnvironment, useLocale } from "@destack/view";
import { Timeout } from "@destack/view/primitives/timer";
import { Button } from "../button/index.ts";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../collapsible/index.ts";
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "../empty/index.ts";
import { Icon } from "../icon/index.ts";
import { Spinner } from "../spinner/index.ts";

/** Why a request is refused: starting, failed, nothing there, no access, no person, no network. */
export type RefusalKind =
    | "starting"
    | "failed"
    | "not-found"
    | "forbidden"
    | "signed-out"
    | "offline";

/** What a person can do next: try again after a while, sign in, or go home. */
export type RefusalAction =
    | { readonly kind: "retry"; readonly after: number }
    | { readonly kind: "sign-in"; readonly url: string }
    | { readonly kind: "home"; readonly url: string };

/** The properties of a refusal: why, of which app and space, its error, and the next step. */
export interface RefusalProperties {
    /** Why the request is refused. */
    readonly kind: RefusalKind;
    /** The app refused, none for the universe itself. */
    readonly package?: { readonly name: string; readonly title: string } | undefined;
    /** The space the app is in, none outside one. */
    readonly space?: { readonly handle: string; readonly title: string } | undefined;
    /** What failed, for people who ask, none unless the app failed. */
    readonly error?: { readonly reason: string; readonly message: string } | undefined;
    /** The next step, none where there is nothing to do. */
    readonly action?: RefusalAction | undefined;
}

/** How each refusal reads and looks, its title and description taking the app's title. */
const KINDS: Readonly<
    Record<
        RefusalKind,
        {
            readonly icon: IconBodies | undefined;
            readonly title: Message<"app">;
            readonly description: Message<"app">;
        }
    >
> = {
    starting: {
        icon: undefined,
        title: t.context("title")`Starting ${argument("app")}`,
        description: t`It takes a few seconds the first time.`,
    },
    failed: {
        icon: warningIcon,
        title: t.context("title")`${argument("app")} couldn't start`,
        description: t`Something went wrong as it started. Try again, or go home.`,
    },
    "not-found": {
        icon: signpostIcon,
        title: t.context("title")`Nothing here`,
        description: t`There is no page at this address, or it moved.`,
    },
    forbidden: {
        icon: lockIcon,
        title: t.context("title")`You don't have access to ${argument("app")}`,
        description: t`Ask whoever runs this space to share it with you.`,
    },
    "signed-out": {
        icon: signInIcon,
        title: t.context("title")`Sign in to continue`,
        description: t`Sign in to open ${argument("app")}.`,
    },
    offline: {
        icon: offlineIcon,
        title: t.context("title")`You're offline`,
        description: t`Destack reconnects as soon as you're back online.`,
    },
};

/** The styles of a refusal: a page of its own, centred, and its error's details. */
const styles = style.create({
    page: {
        display: "grid",
        placeItems: "center",
        minBlockSize: "100dvh",
        padding: space[6],
        backgroundColor: color.background,
    },
    steps: {
        display: "flex",
        flexWrap: "wrap",
        justifyContent: "center",
        gap: space[2],
    },
    error: {
        display: "grid",
        gap: space[2],
        textAlign: "start",
    },
    message: {
        margin: 0,
        padding: space[3],
        borderRadius: radius[2],
        backgroundColor: color.muted,
        color: color.mutedForeground,
        whiteSpace: "pre-wrap",
        overflowWrap: "anywhere",
    },
});

/** Render why a request is refused, as a page of its own, with the next step to take. */
export function Refusal(properties: RefusalProperties): JSX.Element {
    // title it by the app, else by Destack itself
    const locale = useLocale();
    const environment = useEnvironment();
    const kind = () => KINDS[properties.kind];
    const app = () => properties.package?.title ?? "Destack";
    const words = (message: Message<"app">) => locale.render(message, { app: app() });

    // try again on its own once the gateway says the app may be up
    const retrying = new Timeout();
    onCleanup(() => retrying.clear());
    const action = () => properties.action;
    const isRetried = () =>
        properties.kind === "failed" ||
        (properties.kind === "offline" && action()?.kind === "retry");
    const signIn = () => {
        const step = action();

        return step?.kind === "sign-in" ? step : undefined;
    };
    const home = () => {
        const step = action();

        return step?.kind === "home" ? step : undefined;
    };
    const retry = () => environment.navigation.reload();
    if (properties.kind === "starting" && properties.action?.kind === "retry") {
        retrying.set(retry, properties.action.after);
    }

    return (
        <main {...style.attrs(text.sm, styles.page)}>
            <Empty>
                <EmptyHeader>
                    <EmptyMedia variant="icon">
                        <Show when={kind().icon} fallback={<Spinner />}>
                            {(icon) => <Icon icon={icon()} />}
                        </Show>
                    </EmptyMedia>
                    <EmptyTitle render={(attributes) => <h1 {...attributes} />}>
                        {words(kind().title)}
                    </EmptyTitle>
                    <EmptyDescription>{words(kind().description)}</EmptyDescription>
                </EmptyHeader>
                <EmptyContent>
                    {/* The next steps: trying again where it may help, then the gateway's */}
                    <div {...style.attrs(styles.steps)}>
                        <Show when={isRetried()}>
                            <Button onClick={retry}>
                                {locale.render(t.context("action")`Try again`)}
                            </Button>
                        </Show>
                        <Show when={signIn()}>
                            {(step) => (
                                <Button
                                    render={(attributes) => <a {...attributes} href={step().url} />}
                                >
                                    {locale.render(t.context("action")`Sign in`)}
                                </Button>
                            )}
                        </Show>
                        <Show when={home()}>
                            {(step) => (
                                <Button
                                    variant="outline"
                                    render={(attributes) => <a {...attributes} href={step().url} />}
                                >
                                    {locale.render(t.context("action")`Go home`)}
                                </Button>
                            )}
                        </Show>
                    </div>

                    {/* What failed, for people who ask */}
                    <Show when={properties.error}>
                        {(error) => (
                            <Collapsible>
                                <CollapsibleTrigger
                                    render={(attributes) => (
                                        <Button {...attributes} variant="ghost" size="sm" />
                                    )}
                                >
                                    {locale.render(t.context("action")`Details`)}
                                </CollapsibleTrigger>
                                <CollapsibleContent>
                                    <div {...style.attrs(styles.error)}>
                                        <pre {...style.attrs(text.xs, styles.message)}>
                                            {error().reason}
                                            {"\n"}
                                            {error().message}
                                        </pre>
                                    </div>
                                </CollapsibleContent>
                            </Collapsible>
                        )}
                    </Show>
                </EmptyContent>
            </Empty>
        </main>
    );
}
