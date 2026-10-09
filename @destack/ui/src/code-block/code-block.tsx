import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, font, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, omit, useContext } from "@destack/view";
import "./code-block.css";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of a code block and its elements. */
const styles = style.create({
    block: {
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.muted,
        color: color.foreground,
        overflow: "hidden",
    },
    header: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[2],
        minHeight: space[8],
        paddingInlineStart: space[3],
        paddingInlineEnd: space[1],
        borderBottomWidth: stroke.border,
        borderColor: color.border,
        color: color.mutedForeground,
    },
    title: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    content: {
        overflowX: "auto",
        fontFamily: font.code,
        paddingBlock: space[3],
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
        outlineColor: color.ring,
    },
    code: {
        display: "grid",
        minWidth: "max-content",
        counterReset: "line",
    },
    line: {
        minBlockSize: "1lh",
        paddingInline: space[3],
        whiteSpace: "pre",
    },
    numbered: {
        "::before": {
            content: "counter(line)",
            counterIncrement: "line",
            display: "inline-block",
            width: space[6],
            marginInlineEnd: space[3],
            color: color.mutedForeground,
            textAlign: "end",
            userSelect: "none",
        },
    },
});

/** The properties of an element of a code block, the native element's attributes included. */
export type CodeBlockElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** Render a block of code with an optional header, as a figure its title captions. */
export function CodeBlock(
    properties: CodeBlockElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    return renderPart("figure", "code-block", properties, styles.block);
}

/** Render the bar above the code that holds its title and actions, such as a copy button. */
export function CodeBlockHeader(
    properties: CodeBlockElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "code-block-header", properties, [text.footnote, styles.header]);
}

/** Render the code's title, such as its file name, captioning the figure. */
export function CodeBlockTitle(
    properties: CodeBlockElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    return renderPart("figcaption", "code-block-title", properties, styles.title);
}

/** Whether the lines of the nearest code block show their numbers. */
const NumberingContext = createContext(false);

/** The properties of a code block's code, the native `pre` element's attributes included. */
export type CodeBlockContentProperties = CodeBlockElementProperties<
    JSX.HTMLAttributes<HTMLPreElement>
> & {
    /** Whether each line shows its number. */
    readonly isNumbered?: boolean;
    /** The accessible name of the scrolling region, its title when it has one. */
    readonly label?: string;
};

/** Render the code's lines, given as `CodeBlockLine`s, in a scrolling region the keyboard reaches. */
export function CodeBlockContent(properties: CodeBlockContentProperties): JSX.Element {
    const rest = omit(properties, "isNumbered", "label", "children");

    return renderPart("pre", "code-block-content", rest, [text.callout, styles.content], {
        tabindex: 0,
        get "aria-label"() {
            return properties.label ?? "Code";
        },
        get children() {
            return (
                <code {...style.attrs(styles.code)}>
                    <NumberingContext value={properties.isNumbered === true}>
                        {properties.children}
                    </NumberingContext>
                </code>
            );
        },
    });
}

/** The properties of one line of a code block, the native element's attributes included. */
export type CodeBlockLineProperties = CodeBlockElementProperties<
    JSX.HTMLAttributes<HTMLSpanElement>
>;

/** Render one line of a code block's code, numbered when its block numbers lines. */
export function CodeBlockLine(properties: CodeBlockLineProperties): JSX.Element {
    const isNumbered = useContext(NumberingContext);

    return renderPart("span", "code-block-line", properties, [
        styles.line,
        isNumbered && styles.numbered,
    ]);
}
