import * as style from "@destack/style";
import { color, radius, shadow, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of a card and its elements. */
const styles = style.create({
    card: {
        display: "flex",
        flexDirection: "column",
        gap: space[5],
        paddingBlock: space[5],
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
export type CardProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** Render a raised surface that groups related content and actions. */
export function Card(properties: CardProperties): JSX.Element {
    return renderPart("div", "card", properties, styles.card);
}

/** Render the top of a card that holds its title, description and action. */
export function CardHeader(properties: CardProperties): JSX.Element {
    return renderPart("div", "card-header", properties, styles.header);
}

/** Render the title of a card. */
export function CardTitle(properties: CardProperties): JSX.Element {
    return renderPart("div", "card-title", properties, [text.headline, styles.title]);
}

/** Render the description under a card's title. */
export function CardDescription(properties: CardProperties): JSX.Element {
    return renderPart("div", "card-description", properties, [text.footnote, styles.description]);
}

/** Render an action in the top corner of a card's header. */
export function CardAction(properties: CardProperties): JSX.Element {
    return renderPart("div", "card-action", properties, styles.action);
}

/** Render the main content of a card. */
export function CardContent(properties: CardProperties): JSX.Element {
    return renderPart("div", "card-content", properties, styles.content);
}

/** Render the bottom row of a card, usually its actions. */
export function CardFooter(properties: CardProperties): JSX.Element {
    return renderPart("div", "card-footer", properties, styles.footer);
}
