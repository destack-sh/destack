import * as style from "@destack/style";
import { color, motion, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { createContext, merge, omit, useContext } from "solid-js";
import { Separator, type SeparatorProperties } from "../separator/index.ts";

/** The variant and size of an item that sets neither. */
const DEFAULTS: Required<ItemStyleOptions> = { variant: "default", size: "default" };

/** The variant of an item's media that sets none. */
const MEDIA_DEFAULTS: Required<Pick<ItemMediaProperties, "variant">> = { variant: "default" };

/** The styles of an item and its elements. */
const styles = style.create({
    item: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: { default: "transparent", ":focus-visible": color.ring },
        borderRadius: radius[3],
        color: "inherit",
        textDecoration: "none",
        transitionProperty: "background-color, border-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    group: {
        display: "flex",
        flexDirection: "column",
    },
    separator: {
        marginBlock: 0,
    },
    media: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        gap: space[2],
    },
    content: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        gap: space[1],
    },
    title: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        width: "fit-content",
        fontWeight: weight.medium,
    },
    description: {
        display: "-webkit-box",
        overflow: "hidden",
        margin: 0,
        WebkitBoxOrient: "vertical",
        WebkitLineClamp: 2,
        color: color.mutedForeground,
        textWrap: "balance",
    },
    actions: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    edge: {
        display: "flex",
        flexBasis: "100%",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[2],
    },
});

/** The colors of each variant. */
const variants = style.create({
    default: {
        backgroundColor: "transparent",
    },
    outline: {
        borderColor: color.border,
    },
    muted: {
        backgroundColor: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
    },
});

/** The background of each variant as a link, which takes the accent under the pointer. */
const links = style.create({
    default: {
        backgroundColor: {
            default: "transparent",
            ":hover": `color-mix(in oklab, ${color.accent} 50%, transparent)`,
        },
    },
    outline: {
        backgroundColor: {
            default: "transparent",
            ":hover": `color-mix(in oklab, ${color.accent} 50%, transparent)`,
        },
    },
    muted: {
        backgroundColor: {
            default: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
            ":hover": `color-mix(in oklab, ${color.accent} 50%, transparent)`,
        },
    },
});

/** The gap and padding of each size. */
const sizes = style.create({
    default: { gap: space[4], padding: space[4] },
    sm: { gap: space[3], paddingInline: space[4], paddingBlock: space[3] },
});

/** The frame of each media variant. */
const medias = style.create({
    default: {},
    icon: {
        width: size[2],
        height: size[2],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[2],
        backgroundColor: color.muted,
    },
    image: {
        overflow: "hidden",
        width: size[3],
        height: size[3],
        borderRadius: radius[2],
    },
});

/** Whether an item sits in a group, listed as one of its entries. */
const ItemGroupContext = createContext(false);

/** The look of an item. */
export type ItemVariant = "default" | "outline" | "muted";

/** The gap and padding of an item. */
export type ItemSize = "default" | "sm";

/** How an item frames its media: bare, as an icon on a muted tile, or as a cropped picture. */
export type ItemMediaVariant = "default" | "icon" | "image";

/** The properties of an element of an item, the native element's attributes included. */
export type ItemElementProperties<Attributes = JSX.HTMLAttributes<HTMLDivElement>> = Omit<
    Attributes,
    "class" | "style"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** The variant and size of an item's styles. */
export interface ItemStyleOptions {
    /** The look, default by default. */
    readonly variant?: ItemVariant;
    /** The gap and padding, default by default. */
    readonly size?: ItemSize;
}

/** The properties of an item, the native element's attributes included. */
export interface ItemProperties extends ItemElementProperties, ItemStyleOptions {}

/** The properties of an item's media, the native element's attributes included. */
export interface ItemMediaProperties extends ItemElementProperties {
    /** The frame, default by default. */
    readonly variant?: ItemMediaVariant;
}

/** Return the StyleX styles of an item in a variant and size, for links that look like items. */
export function itemStyle(options: ItemStyleOptions): style.Styles {
    const variant = options.variant ?? DEFAULTS.variant;

    return [
        text.callout,
        styles.item,
        variants[variant],
        links[variant],
        sizes[options.size ?? DEFAULTS.size],
    ];
}

/** Render a row of media, content and actions, listed as an entry inside a group. */
export function Item(properties: ItemProperties): JSX.Element {
    // list the item when a group holds it
    const isGrouped = useContext(ItemGroupContext);
    const item = merge(DEFAULTS, properties);
    const rest = omit(item, "variant", "size", "style");

    return (
        <div
            data-slot="item"
            data-variant={item.variant}
            data-size={item.size}
            role={isGrouped ? "listitem" : undefined}
            {...rest}
            {...style.attrs(
                text.callout,
                styles.item,
                variants[item.variant],
                sizes[item.size],
                item.style,
            )}
        />
    );
}

/** Render items as a list. */
export function ItemGroup(properties: ItemElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <ItemGroupContext value={true}>
            <div
                data-slot="item-group"
                role="list"
                {...rest}
                {...style.attrs(styles.group, properties.style)}
            />
        </ItemGroupContext>
    );
}

/** Render a line between the items of a group. */
export function ItemSeparator(properties: SeparatorProperties): JSX.Element {
    return (
        <Separator
            data-slot="item-separator"
            {...properties}
            orientation="horizontal"
            style={[styles.separator, properties.style]}
        />
    );
}

/** Render an item's icon, avatar or picture. */
export function ItemMedia(properties: ItemMediaProperties): JSX.Element {
    const media = merge(MEDIA_DEFAULTS, properties);
    const rest = omit(media, "variant", "style");

    return (
        <div
            data-slot="item-media"
            data-variant={media.variant}
            {...rest}
            {...style.attrs(styles.media, medias[media.variant], media.style)}
        />
    );
}

/** Render the title and description of an item, taking the row's free width. */
export function ItemContent(properties: ItemElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="item-content"
            {...rest}
            {...style.attrs(styles.content, properties.style)}
        />
    );
}

/** Render the title of an item. */
export function ItemTitle(properties: ItemElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div data-slot="item-title" {...rest} {...style.attrs(styles.title, properties.style)} />
    );
}

/** Render the description under an item's title, cut to two lines. */
export function ItemDescription(
    properties: ItemElementProperties<JSX.HTMLAttributes<HTMLParagraphElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <p
            data-slot="item-description"
            {...rest}
            {...style.attrs(styles.description, properties.style)}
        />
    );
}

/** Render the buttons at the end of an item. */
export function ItemActions(properties: ItemElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="item-actions"
            {...rest}
            {...style.attrs(styles.actions, properties.style)}
        />
    );
}

/** Render a full-width row above an item's media and content. */
export function ItemHeader(properties: ItemElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div data-slot="item-header" {...rest} {...style.attrs(styles.edge, properties.style)} />
    );
}

/** Render a full-width row below an item's media and content. */
export function ItemFooter(properties: ItemElementProperties): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div data-slot="item-footer" {...rest} {...style.attrs(styles.edge, properties.style)} />
    );
}
