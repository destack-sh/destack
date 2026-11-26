import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type Accessor, createContext, type JSX, omit, useContext } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The styles of a card and its elements. */
const styles = style.create({
    card: {
        display: "flex",
        flexDirection: "column",
        gap: space[5],
        paddingBlock: space[5],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.card,
        color: color.foreground,
    },
    small: {
        gap: space[3],
        paddingBlock: space[3],
    },
    header: {
        display: "grid",
        gridTemplateColumns: "1fr auto",
        gridAutoRows: "min-content",
        alignItems: "start",
        rowGap: space[2],
    },
    title: {
        gridColumnStart: 1,
        fontWeight: weight.medium,
    },
    description: {
        gridColumnStart: 1,
        color: color.mutedForeground,
    },
    action: {
        gridColumnStart: 2,
        gridRow: "1 / span 2",
        alignSelf: "start",
        justifySelf: "end",
        paddingInlineStart: space[2],
    },
    footer: {
        display: "flex",
        alignItems: "center",
    },
});

/** The inline padding of a card's header, content and footer at each size. */
const insets = style.create({
    default: { paddingInline: space[5] },
    sm: { paddingInline: space[3] },
});

/** The nearest card, null outside one. */
const CardContext = createContext<CardControl | null>(null);

/** A card as its parts read it. */
export interface CardControl {
    /** The card's size, which its parts' insets follow. */
    readonly size: Accessor<CardSize>;
}

/** Read the nearest card, refusing a part outside one. */
export function useCard(): CardControl {
    const card = useContext(CardContext);
    if (card === null) {
        throw new TypeError("a card part needs a card around it");
    }

    return card;
}

/** The size of a card: the default, or small for a dense one such as a stat tile. */
export type CardSize = keyof typeof CARD_SIZES;

/** The properties of a card's element, the native element's attributes included. */
export type CardElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The sizes of a card. */
const CARD_SIZES = {
    default: { title: t.context("option")`Default` },
    sm: {
        title: t.context("option")`Small`,
        description: t`Dense, such as a stat tile.`,
    },
};

/** The properties people change in a card's controls and inspector. */
const CARD_PROPERTIES = {
    size: field
        .enum(CARD_SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How much room the card leaves around its content.`)
        .default("default"),
};

/** The properties of a card besides its declared ones: its native attributes. */
export interface CardAttributes extends CardElementProperties {}

/** Render a card: a bordered surface that groups related content and actions. */
export const Card = defineComponent({
    name: "card",
    properties: CARD_PROPERTIES,
    render(properties: RenderProperties<CardAttributes, typeof CARD_PROPERTIES>): JSX.Element {
        // share the size with the parts, following its changes
        const size = (): CardSize => properties.size;

        return (
            <CardContext value={{ size }}>
                {renderPart(
                    "div",
                    "card",
                    omit(properties, "size"),
                    () => [styles.card, size() === "sm" && styles.small],
                    {
                        get "data-size"() {
                            return size();
                        },
                    },
                )}
            </CardContext>
        );
    },
});

/** The properties of a card, its native attributes included. */
export type CardProperties = ComponentProperties<typeof Card>;

/** Render the top of a card with its title, description and action. */
export function CardHeader(properties: CardElementProperties): JSX.Element {
    const { size } = useCard();

    return renderPart("div", "card-header", properties, () => [styles.header, insets[size()]]);
}

/** Render the title of a card. */
export function CardTitle(properties: CardElementProperties): JSX.Element {
    return renderPart("div", "card-title", properties, [text.base, styles.title]);
}

/** Render the description under a card's title. */
export function CardDescription(properties: CardElementProperties): JSX.Element {
    return renderPart("div", "card-description", properties, [text.xs, styles.description]);
}

/** Render an action in the top corner of a card's header. */
export function CardAction(properties: CardElementProperties): JSX.Element {
    return renderPart("div", "card-action", properties, styles.action);
}

/** Render the main content of a card. */
export function CardContent(properties: CardElementProperties): JSX.Element {
    const { size } = useCard();

    return renderPart("div", "card-content", properties, () => insets[size()]);
}

/** Render the bottom row of a card, usually its actions. */
export function CardFooter(properties: CardElementProperties): JSX.Element {
    const { size } = useCard();

    return renderPart("div", "card-footer", properties, () => [styles.footer, insets[size()]]);
}
