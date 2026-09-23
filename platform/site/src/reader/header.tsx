import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { type JSX, Show } from "@destack/view";

/// Render a page title, its optional description, and any metadata below it.
export function PageHeader(props: {
    /// The page title.
    title: string;
    /// The page kind; articles leave more room below their opening.
    variant: "article" | "chapter" | "reference";
    /// The optional lead below the title.
    description?: string;
    /// The optional metadata row.
    children?: JSX.Element;
}) {
    return (
        <header {...stylex.attrs(styles.header, props.variant === "article" && styles.article)}>
            <h1 {...stylex.attrs(styles.title)}>{props.title}</h1>
            <Show when={props.description}>
                <p {...stylex.attrs(styles.description)}>{props.description}</p>
            </Show>
            {props.children}
        </header>
    );
}

const styles = stylex.create({
    header: {
        display: "grid",
        gap: "1rem",
        paddingBottom: "1.5rem",
        paddingTop: "2.5rem",
    },
    article: {
        marginBottom: "1rem",
    },
    title: {
        color: color.foreground,
        fontSize: "clamp(2.25rem, 3.8vw, 3rem)",
        fontWeight: 500,
        letterSpacing: "-0.025em",
        lineHeight: 1.08,
        margin: 0,
        overflowWrap: "anywhere",
        textWrap: "balance",
    },
    description: {
        color: color.mutedForeground,
        fontSize: "1.25rem",
        lineHeight: 1.5,
        margin: 0,
        maxWidth: "38rem",
    },
});
