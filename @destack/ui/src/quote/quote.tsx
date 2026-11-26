import quotesIcon from "@destack/icon/phosphor/quotes";
import xIcon from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { radius, size, space } from "@destack/theme/tokens.stylex";
import { type JSX, omit, useLocale } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import { contentOr, type ElementPartProperties, renderPart } from "../part/index.ts";

/** The lines of a quote before it is cut. */
const LINES = 3;

/** The styles of a quote and its parts. */
const styles = style.create({
    quote: {
        display: "flex",
        alignItems: "flex-start",
        gap: space[2],
        boxSizing: "border-box",
        width: "100%",
        minWidth: 0,
        paddingBlock: space[2],
        paddingInline: space[3],
        borderRadius: radius[5],
        backgroundColor: "color-mix(in oklab, currentColor 8%, transparent)",
        color: "inherit",
        textDecorationLine: "none",
    },
    icon: {
        display: "inline-flex",
        flexShrink: 0,
        height: "1lh",
        alignItems: "center",
    },
    content: {
        flexGrow: 1,
        minWidth: 0,
        display: "-webkit-box",
        WebkitBoxOrient: "vertical",
        WebkitLineClamp: LINES,
        overflow: "hidden",
        overflowWrap: "anywhere",
        textAlign: "start",
    },
    dismiss: {
        marginInlineEnd: `calc(-1 * ${space[2]})`,
        marginBlock: `calc((1lh - ${size[1]}) / 2)`,
        color: "inherit",
    },
});

/** The properties of an element of a quote, the native element's attributes included. */
export type QuoteElementProperties<Target extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** Render what something answers, over it, such as over a message or its composer's text. */
export function Quote(properties: QuoteElementProperties): JSX.Element {
    return renderPart("div", "quote", properties, [text.sm, styles.quote]);
}

/** Render what marks a quote, a quotation mark by default. */
export function QuoteIcon(properties: QuoteElementProperties<HTMLSpanElement>): JSX.Element {
    const rest = omit(properties, "children");

    return renderPart("span", "quote-icon", rest, styles.icon, {
        "aria-hidden": "true" as const,
        get children() {
            return contentOr(properties, () => <Icon icon={quotesIcon} />);
        },
    });
}

/** Render the words of a quote, cut after some lines. */
export function QuoteContent(properties: QuoteElementProperties): JSX.Element {
    return renderPart("div", "quote-content", properties, styles.content);
}

/** Render the button that takes a quote back. */
export function QuoteDismiss(properties: ButtonProperties): JSX.Element {
    const locale = useLocale();

    return (
        <Button
            isIconOnly
            data-slot="quote-dismiss"
            variant="ghost"
            size="sm"
            shape="round"
            aria-label={locale.render(t.context("action")`Remove quote`)}
            {...properties}
            xstyle={[styles.dismiss, properties.xstyle]}
        >
            {contentOr(properties, () => (
                <Icon icon={xIcon} />
            ))}
        </Button>
    );
}
