import * as style from "@destack/style";
import { color, radius, size, space, weight, width } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The variant of an empty state's media that sets none. */
const DEFAULTS: Required<Pick<EmptyMediaProperties, "variant">> = { variant: "default" };

/** The styles of an empty state and its elements. */
const styles = style.create({
    empty: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: space[6],
        minWidth: 0,
        padding: `clamp(${space[6]}, 6cqi, ${space[9]})`,
        borderRadius: radius[4],
        textAlign: "center",
        textWrap: "balance",
    },
    header: {
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: space[2],
        maxWidth: width.prose,
        textAlign: "center",
    },
    media: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        marginBottom: space[2],
    },
    title: {
        fontWeight: weight.medium,
    },
    description: {
        color: color.mutedForeground,
    },
    content: {
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: space[4],
        width: "100%",
        minWidth: 0,
        maxWidth: width.prose,
        textWrap: "balance",
    },
});

/** The frame of each media variant. */
const medias = style.create({
    default: {},
    icon: {
        width: size[4],
        height: size[4],
        borderRadius: radius[4],
        backgroundColor: color.muted,
        color: color.foreground,
    },
});

/** How an empty state frames its media: bare, or as an icon on a muted tile. */
export type EmptyMediaVariant = "default" | "icon";

/** The properties of an element of an empty state, the native element's attributes included. */
export type EmptyElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of an empty state's media, the native element's attributes included. */
export interface EmptyMediaProperties extends EmptyElementProperties {
    /** The frame, default by default. */
    readonly variant?: EmptyMediaVariant;
}

/** Render what a list, search or page shows when it has nothing yet, with the actions that fill it. */
export function Empty(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty", properties, styles.empty);
}

/** Render the top of an empty state that holds its media, title and description. */
export function EmptyHeader(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-header", properties, styles.header);
}

/** Render an empty state's icon, avatar or picture. */
export function EmptyMedia(properties: EmptyMediaProperties): JSX.Element {
    const media = merge(DEFAULTS, properties);
    const rest = omit(media, "variant", "xstyle", "style");

    return (
        <div
            data-slot="empty-media"
            data-variant={media.variant}
            {...rest}
            {...style.attributes([styles.media, medias[media.variant], media.xstyle], media.style)}
        />
    );
}

/** Render the title of an empty state. */
export function EmptyTitle(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-title", properties, [text.headline, styles.title]);
}

/** Render the description under an empty state's title. */
export function EmptyDescription(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-description", properties, [text.footnote, styles.description]);
}

/** Render the actions and links under an empty state's header. */
export function EmptyContent(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-content", properties, [text.footnote, styles.content]);
}
