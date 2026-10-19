import type { Element, ElementContent, Root } from "hast";
import { Icon } from "@destack/icon";
import arrowDown from "@destack/icon/phosphor/arrow-down";
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
import { CopyButton } from "../layout/copy";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@destack/ui/table";

import { palette } from "../palette.stylex";
import { media } from "@destack/style/media.stylex";
import { createEffect, createSignal } from "@destack/view";
import { ImageMetadata, Picture } from "@destack/view/media";
import { createReducedMotion } from "@destack/view/motion";

/** The components that stand in for the elements of the site's articles. */
export const blocks: ContentComponents = {
    pre: Listing,
    figure: Figure,
    aside: Aside,
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

/** Render a fence as a listing. */
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

/** Render a figure by its kind. */
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

    // frame an image or a looping animation at its authored width, numbered under its caption
    if (kind === "image" || kind === "animation") {
        const image = imageOf(properties);
        const alt = attribute(properties, "data-alt") ?? "";

        return (
            <figure {...style.attrs(styles.figure)}>
                <div
                    style={
                        width === undefined ? undefined : { "max-width": `min(100%, ${width}px)` }
                    }
                    {...style.attrs(styles.frame)}
                >
                    {kind === "animation" ? (
                        <Animation
                            source={attribute(properties, "data-src") ?? ""}
                            alt={alt}
                            poster={image}
                        />
                    ) : image === undefined ? (
                        <img
                            src={attribute(properties, "data-src")}
                            alt={alt}
                            loading="lazy"
                            decoding="async"
                            {...style.attrs(styles.media)}
                        />
                    ) : (
                        <Picture
                            src={image}
                            alt={alt}
                            sizes={sizesOf(width)}
                            xstyle={styles.media}
                        />
                    )}
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

/** Render an aside: an abstract with its skip link past the essay, or any other aside as it is. */
function Aside(properties: ContentElementProperties) {
    // leave an aside of no kind to the prose
    if (attribute(properties, "data-kind") !== "abstract") {
        return <aside>{properties.children}</aside>;
    }

    // read the heading the abstract skips to, and the words of its link
    const to = attribute(properties, "data-to");
    const skip = attribute(properties, "data-skip");

    return (
        <aside aria-label="Abstract" {...style.attrs(styles.abstract)}>
            <span {...style.attrs(styles.abstractLabel)}>Abstract</span>
            <div {...style.attrs(styles.abstractBody)}>{properties.children}</div>
            {to === undefined ? undefined : (
                <a href={to} {...style.attrs(styles.skip)}>
                    {skip}
                    <Icon icon={arrowDown} xstyle={styles.skipIcon} />
                </a>
            )}
        </aside>
    );
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

/** Play a video as a silent looping animation, held on its poster frame while motion is reduced. */
function Animation(properties: { source: string; alt: string; poster: ImageMetadata | undefined }) {
    // play while motion runs, and hold the poster frame while it is reduced
    const [video, setVideo] = createSignal<HTMLVideoElement>();
    const isStill = createReducedMotion(video);
    createEffect(
        () => ({ element: video(), still: isStill() }),
        ({ element, still }) => {
            if (element === undefined) {
                return;
            }
            if (still) {
                element.pause();
            } else {
                void element.play();
            }
        },
    );

    return (
        <video
            ref={setVideo}
            muted
            loop
            playsinline
            preload="metadata"
            aria-label={properties.alt}
            src={properties.source}
            poster={properties.poster?.src}
            width={properties.poster?.width}
            height={properties.poster?.height}
            {...style.attrs(styles.media)}
        />
    );
}

/** Read the published image a figure carries, its source or poster frame, none if external. */
function imageOf(properties: ContentElementProperties): ImageMetadata | undefined {
    // read none for an external image, else the published image's description
    const serialized = attribute(properties, "data-image");
    if (serialized === undefined) {
        return undefined;
    }
    const value: unknown = JSON.parse(serialized);

    return ImageMetadata.parse(value);
}

/** Describe the width an image takes: its authored width, or the column below it. */
function sizesOf(width: string | undefined): string {
    return width === undefined ? "100vw" : `(min-width: ${width}px) ${width}px, 100vw`;
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
    abstract: {
        borderBlockColor: color.border,
        borderBlockStyle: "solid",
        borderBlockWidth: stroke.border,
        borderInlineWidth: 0,
        display: "grid",
        justifyItems: "start",
        marginBlockEnd: space[6],
        marginBlockStart: 0,
        paddingBlock: space[6],
        rowGap: space[3],
    },
    abstractLabel: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.6875rem",
        letterSpacing: "0.08em",
        textTransform: "uppercase",
    },
    abstractBody: {
        color: color.foreground,
        display: "grid",
        fontSize: { default: "1.25rem", [media.maxSm]: "1.125rem" },
        justifySelf: "stretch",
        lineHeight: 1.55,
        rowGap: space[4],
    },
    skip: {
        alignItems: "center",
        color: color.primary,
        columnGap: space[1],
        display: "inline-flex",
        fontFamily: font.code,
        fontSize: "0.75rem",
        letterSpacing: "0.08em",
        textDecorationLine: {
            default: "none",
            ":hover": { default: null, [media.hover]: "underline" },
        },
        textTransform: "uppercase",
        textUnderlineOffset: "0.25em",
    },
    skipIcon: {
        height: "0.875rem",
        width: "0.875rem",
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
    head: { display: { default: null, [media.maxMd]: "none" } },
    row: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: { default: null, [media.maxMd]: "block" },
        paddingBlock: { default: null, [media.maxMd]: space[2] },
    },
    cell: {
        "::before": {
            color: color.foreground,
            content: { default: null, [media.maxMd]: "attr(data-label)" },
            fontWeight: weight.semibold,
        },
        borderBottomWidth: { default: null, [media.maxMd]: 0 },
        display: { default: null, [media.maxMd]: "grid" },
        gap: { default: null, [media.maxMd]: space[3] },
        gridTemplateColumns: {
            default: null,
            [media.maxMd]: "minmax(6.5rem, 0.4fr) minmax(0, 1fr)",
        },
        overflowWrap: { default: null, [media.maxMd]: "anywhere" },
        paddingBlock: { default: null, [media.maxMd]: space[1] },
        paddingInline: { default: null, [media.maxMd]: space[3] },
    },
});
