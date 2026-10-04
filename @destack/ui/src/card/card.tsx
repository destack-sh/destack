import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";

/** The styles of a card and its elements. */
const styles = style.create({
    card: {
        display: "flex",
        flexDirection: "column",
        gap: space[5],
        paddingBlock: space[5],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[5],
        backgroundColor: color.card,
        color: color.cardForeground,
        boxShadow: shadow.raised,
    },
    header: {
        display: "grid",
        gridTemplateColumns: "1fr auto",
        gridAutoRows: "min-content",
        alignItems: "start",
        rowGap: space[2],
        paddingInline: space[5],
    },
    title: {
        fontWeight: weight.semibold,
        lineHeight: 1,
    },
    description: {
        color: color.mutedForeground,
    },
    action: {
        gridColumnStart: 2,
        gridRow: "1 / span 2",
        alignSelf: "start",
        justifySelf: "end",
        paddingInlineStart: space[2],
    },
    content: {
        paddingInline: space[5],
    },
    footer: {
        display: "flex",
        alignItems: "center",
        paddingInline: space[5],
    },
});

/** The properties of a card or one of its elements, the native element's attributes included. */
export interface CardProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
}

/** Render a raised surface that groups related content and actions. */
export function Card(properties: CardProperties): JSX.Element {
    const rest = omit(properties, "style");

    return <div data-slot="card" {...rest} {...style.attrs(styles.card, properties.style)} />;
}

/** Render the top of a card that holds its title, description and action. */
export function CardHeader(properties: CardProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div data-slot="card-header" {...rest} {...style.attrs(styles.header, properties.style)} />
    );
}

/** Render the title of a card. */
export function CardTitle(properties: CardProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="card-title"
            {...rest}
            {...style.attrs(text.headline, styles.title, properties.style)}
        />
    );
}

/** Render the description under a card's title. */
export function CardDescription(properties: CardProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="card-description"
            {...rest}
            {...style.attrs(text.footnote, styles.description, properties.style)}
        />
    );
}

/** Render an action in the top corner of a card's header. */
export function CardAction(properties: CardProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div data-slot="card-action" {...rest} {...style.attrs(styles.action, properties.style)} />
    );
}

/** Render the main content of a card. */
export function CardContent(properties: CardProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="card-content"
            {...rest}
            {...style.attrs(styles.content, properties.style)}
        />
    );
}

/** Render the bottom row of a card, usually its actions. */
export function CardFooter(properties: CardProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div data-slot="card-footer" {...rest} {...style.attrs(styles.footer, properties.style)} />
    );
}
