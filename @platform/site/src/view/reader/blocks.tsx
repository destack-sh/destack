import type { Element, ElementContent, Root } from "hast";
import { Icon } from "@destack/icon";
import play from "@destack/icon/phosphor/play";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, font, space, stroke, weight } from "@destack/theme/tokens.stylex";
import {
    CodeBlock,
    CodeBlockContent,
    CodeBlockHeader,
    CodeBlockLine,
    CodeBlockTitle,
} from "@destack/ui/code-block";
import {
    Content,
    type ContentComponents,
    type ContentElementProperties,
} from "@destack/ui/content";
import { CopyButton } from "@destack/ui/copy-button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@destack/ui/table";

import { palette } from "../palette.stylex";

/** The media query for phone-width screens, where table rows stack into labelled cells. */
const phone = "@media (max-width: 767px)";

/** The components that stand in for the elements of the site's articles: listings, figures and tables. */
export const blocks: ContentComponents = {
    pre: Listing,
    figure: Figure,
    table: (properties) => <Table>{properties.children}</Table>,
    thead: (properties) => <TableHeader xstyle={styles.head}>{properties.children}</TableHeader>,
    tbody: (properties) => <TableBody>{properties.children}</TableBody>,
    tr: (properties) => <TableRow xstyle={styles.row}>{properties.children}</TableRow>,
    th: (properties) => <TableHead>{properties.children}</TableHead>,
    td: (properties) => (
        <TableCell data-label={attribute(properties, "data-label")} xstyle={styles.cell}>
            {properties.children}
        </TableCell>
    ),
};

/** Render a fence as a listing: a numbered, highlighted code block under its title and format, or a Mermaid diagram's source. */
function Listing(properties: ContentElementProperties) {
    // read the fence's language, title and format, and its code
    const language = attribute(properties, "data-language");
    const title = attribute(properties, "data-title");
    const format = attribute(properties, "data-format");
    const code = elementAt(properties.node, 0);
    const source = textOf(code);

    // keep a Mermaid diagram's source for the reader to draw
    if (language === "mermaid") {
        return (
            <figure {...style.attrs(styles.diagram)}>
                {title === undefined ? undefined : (
                    <figcaption {...style.attrs(text.subheadline, styles.diagramTitle)}>
                        {title}
                    </figcaption>
                )}
                <div
                    data-mermaid
                    tabindex="0"
                    role="region"
                    aria-label={title ?? "Diagram"}
                    {...style.attrs(styles.drawing)}
                >
                    {source}
                </div>
            </figure>
        );
    }

    // show the code one numbered line per row, a plain diagram unnumbered
    return (
        <CodeBlock xstyle={styles.listing}>
            {title === undefined ? undefined : (
                <CodeBlockHeader>
                    <CodeBlockTitle>{title}</CodeBlockTitle>
                    <span {...style.attrs(text.footnote, styles.format)}>{format}</span>
                    <CopyButton value={source} />
                </CodeBlockHeader>
            )}
            <CodeBlockContent
                isNumbered={language !== "diagram"}
                label={title ?? language ?? "Code"}
            >
                {code.children.map((line) => (
                    <CodeBlockLine>
                        <Content tree={rootOf(line.type === "element" ? line.children : [line])} />
                    </CodeBlockLine>
                ))}
            </CodeBlockContent>
        </CodeBlock>
    );
}

/** Render a figure by its kind: a framed image or video numbered under its caption, a diagram, or a credited quotation. */
function Figure(properties: ContentElementProperties) {
    // read the figure's kind, number, width and caption
    const kind = attribute(properties, "data-kind");
    const label = attribute(properties, "data-label");
    const width = attribute(properties, "data-width");
    const caption = properties.node.children.find(
        (child): child is Element => child.type === "element" && child.tagName === "figcaption",
    );
    const captionText =
        caption === undefined ? undefined : <Content tree={rootOf(caption.children)} />;

    // frame an image at its authored width, numbered under its caption
    if (kind === "image") {
        return (
            <figure {...style.attrs(styles.figure)}>
                <div
                    style={
                        width === undefined ? undefined : { "max-width": `min(100%, ${width}px)` }
                    }
                    {...style.attrs(styles.frame)}
                >
                    <img
                        src={attribute(properties, "data-src")}
                        alt={attribute(properties, "data-alt") ?? ""}
                        loading="lazy"
                        decoding="async"
                        {...style.attrs(styles.media)}
                    />
                </div>
                <figcaption {...style.attrs(text.subheadline, styles.caption)}>
                    <span {...style.attrs(styles.label)}>{label}</span>
                    <span>{captionText}</span>
                </figcaption>
            </figure>
        );
    }
    // frame a video: a YouTube preview the reader plays on click, or the file itself
    else if (kind === "video") {
        const source = attribute(properties, "data-src") ?? "";
        const embed = attribute(properties, "data-embed");
        const title = attribute(properties, "data-title") ?? "";
        const poster = attribute(properties, "data-poster");

        return (
            <figure {...style.attrs(styles.figure)}>
                <div
                    style={
                        width === undefined ? undefined : { "max-width": `min(100%, ${width}px)` }
                    }
                    {...style.attrs(styles.frame)}
                >
                    {embed === undefined ? (
                        <video
                            controls
                            playsinline
                            preload="none"
                            aria-label={title}
                            src={source}
                            poster={poster}
                            {...style.attrs(styles.player)}
                        >
                            <a href={source}>{title}</a>
                        </video>
                    ) : (
                        <a
                            href={source}
                            data-video-src={embed}
                            data-video-title={title}
                            aria-label={`Play ${title}`}
                            {...style.attrs(style.defaultMarker(), styles.player, styles.preview)}
                        >
                            <img
                                src={poster}
                                alt=""
                                loading="lazy"
                                decoding="async"
                                {...style.attrs(styles.cover)}
                            />
                            <span aria-hidden="true" {...style.attrs(styles.play)}>
                                <Icon icon={play} weight="fill" />
                            </span>
                        </a>
                    )}
                </div>
                <figcaption {...style.attrs(text.subheadline, styles.caption, styles.videoCaption)}>
                    <span>
                        {title}
                        {caption === undefined || caption.children.length === 0 ? undefined : (
                            <br />
                        )}
                        {captionText}
                    </span>
                    <a
                        href={source}
                        aria-label={`Open ${title}${embed === undefined ? "" : " on YouTube"}`}
                    >
                        {embed === undefined ? "Open video" : "YouTube"} ↗
                    </a>
                </figcaption>
            </figure>
        );
    }
    // frame a plain diagram under its numbered caption
    else if (kind === "diagram") {
        const listing = properties.node.children.filter((child) => child !== caption);

        return (
            <figure {...style.attrs(styles.diagram)}>
                <figcaption {...style.attrs(text.subheadline, styles.diagramTitle)}>
                    <span {...style.attrs(styles.label)}>{label}</span>
                    {captionText}
                </figcaption>
                <Content tree={rootOf(listing)} components={blocks} />
            </figure>
        );
    }

    // leave a credited quotation, and any other figure, to the prose
    return <figure>{properties.children}</figure>;
}

/** Read one of an element's attributes as text, undefined when absent. */
function attribute(properties: ContentElementProperties, name: string): string | undefined {
    const value = properties[name];

    return typeof value === "string" ? value : undefined;
}

/** Read the element child of an element at a position, failing when the build left it out. */
function elementAt(node: Element, index: number): Element {
    const child = node.children.filter(
        (candidate): candidate is Element => candidate.type === "element",
    )[index];
    if (child === undefined) {
        throw new TypeError(`a ${node.tagName} has no element child at ${index}`);
    }

    return child;
}

/** Read the text of an element, its descendants' text joined. */
function textOf(node: ElementContent): string {
    if (node.type === "text") {
        return node.value;
    }

    return node.type === "element" ? node.children.map(textOf).join("") : "";
}

/** Wrap nodes as a tree to render on their own. */
function rootOf(children: readonly ElementContent[]): Root {
    return { type: "root", children: [...children] };
}

/** The styles of the article blocks. */
const styles = style.create({
    listing: {
        marginBlock: space[2],
    },
    format: {
        color: color.mutedForeground,
        fontFamily: font.code,
        marginInlineStart: "auto",
    },
    diagram: {
        backgroundColor: color.muted,
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        marginBlock: space[2],
        overflowX: "auto",
        overscrollBehaviorInline: "contain",
    },
    diagramTitle: {
        alignItems: "baseline",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.foreground,
        display: "flex",
        fontWeight: weight.medium,
        gap: space[3],
        marginBlockStart: 0,
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    drawing: {
        padding: space[6],
    },
    figure: {
        marginBlock: space[4],
        marginInline: "auto",
        maxWidth: "100%",
    },
    frame: {
        backgroundColor: color.muted,
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: stroke.border,
        marginInline: "auto",
        padding: "0.375rem",
    },
    media: {
        display: "block",
        width: "100%",
    },
    caption: {
        color: color.foreground,
        display: "flex",
        gap: space[4],
        justifyContent: "center",
        marginBlockStart: space[3],
    },
    videoCaption: {
        justifyContent: "space-between",
    },
    label: {
        flexShrink: 0,
        fontWeight: weight.semibold,
    },
    player: {
        aspectRatio: "16 / 9",
        backgroundColor: palette.night,
        borderWidth: 0,
        display: "block",
        width: "100%",
    },
    preview: {
        color: palette.cream,
        position: "relative",
        textDecoration: "none",
    },
    cover: {
        display: "block",
        height: "100%",
        objectFit: "cover",
        width: "100%",
    },
    play: {
        alignItems: "center",
        backgroundColor: {
            default: `color-mix(in srgb, ${palette.ink} 90%, transparent)`,
            [style.when.ancestor(":hover")]: color.primary,
        },
        borderColor: `color-mix(in srgb, ${palette.cream} 65%, transparent)`,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        fontSize: "1.5rem",
        height: "3.5rem",
        insetBlockStart: "50%",
        insetInlineStart: "50%",
        justifyItems: "center",
        position: "absolute",
        transform: "translate(-50%, -50%)",
        width: "3.5rem",
    },
    head: {
        [phone]: { display: "none" },
    },
    row: {
        [phone]: {
            borderBottomColor: color.border,
            borderBottomStyle: "solid",
            borderBottomWidth: stroke.border,
            display: "block",
            paddingBlock: space[2],
        },
    },
    cell: {
        [phone]: {
            borderBottomWidth: 0,
            display: "grid",
            gap: space[3],
            gridTemplateColumns: "minmax(6.5rem, 0.4fr) minmax(0, 1fr)",
            overflowWrap: "anywhere",
            paddingBlock: space[1],
            paddingInline: space[3],
        },
        "::before": {
            color: color.foreground,
            content: { default: null, [phone]: "attr(data-label)" },
            fontWeight: weight.semibold,
        },
    },
});
