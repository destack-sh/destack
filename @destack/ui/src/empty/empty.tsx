import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import { color, radius, size, space, weight, width } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

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
export type EmptyMediaVariant = keyof typeof EMPTY_MEDIA_VARIANTS;

/** The properties of an element of an empty state, the native element's attributes included. */
export type EmptyElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The frames of an empty state's media. */
const EMPTY_MEDIA_VARIANTS = {
    default: {
        title: t.context("option")`Default`,
        description: t`Bare, as an illustration.`,
    },
    icon: {
        title: t.context("option")`Icon`,
        description: t`An icon on a muted tile.`,
    },
};

/** The properties people change in an empty state's media's controls and inspector. */
const EMPTY_MEDIA_PROPERTIES = {
    variant: field
        .enum(EMPTY_MEDIA_VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the media stands bare or as an icon on a tile.`)
        .default("default"),
};

/** The properties of an empty state's media besides its declared ones: its native attributes. */
export interface EmptyMediaAttributes extends EmptyElementProperties {}

/** Render what a list, search or page shows when it has nothing yet. */
export function Empty(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty", properties, styles.empty);
}

/** Render the top of an empty state with its media, title and description. */
export function EmptyHeader(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-header", properties, styles.header);
}

/** Render an empty state's icon, avatar or picture. */
export const EmptyMedia = defineComponent({
    name: "emptyMedia",
    properties: EMPTY_MEDIA_PROPERTIES,
    render(
        properties: RenderProperties<EmptyMediaAttributes, typeof EMPTY_MEDIA_PROPERTIES>,
    ): JSX.Element {
        const rest = omit(properties, "variant");

        return renderPart(
            "div",
            "empty-media",
            rest,
            () => [styles.media, medias[properties.variant]],
            {
                get "data-variant"() {
                    return properties.variant;
                },
            },
        );
    },
});

/** The properties of an empty state's media, its native attributes included. */
export type EmptyMediaProperties = ComponentProperties<typeof EmptyMedia>;

/** Render the title of an empty state. */
export function EmptyTitle(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-title", properties, [text.lg, styles.title]);
}

/** Render the description under an empty state's title. */
export function EmptyDescription(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-description", properties, [text.xs, styles.description]);
}

/** Render the actions and links under an empty state's header. */
export function EmptyContent(properties: EmptyElementProperties): JSX.Element {
    return renderPart("div", "empty-content", properties, [text.xs, styles.content]);
}
