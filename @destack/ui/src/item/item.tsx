import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createContext, type JSX, omit, useContext } from "@destack/view";
import {
    type ComponentProperties,
    type DeclaredProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { field } from "@destack/object";
import { t } from "@destack/locale";
import { Separator, type SeparatorProperties } from "../separator/index.ts";
import { renderPart, type Render } from "../part/index.ts";

/** The looks of an item. */
const ITEM_VARIANTS = {
    default: { title: t.context("option")`Default` },
    outline: { title: t.context("option")`Outline` },
    muted: { title: t.context("option")`Muted` },
};

/** The gaps and paddings of an item. */
const ITEM_SIZES = {
    default: { title: t.context("option")`Default` },
    sm: { title: t.context("option")`Small` },
};

/** The frames of an item's media. */
const ITEM_MEDIA_VARIANTS = {
    default: {
        title: t.context("option")`Default`,
        description: t`Bare, as an avatar.`,
    },
    icon: {
        title: t.context("option")`Icon`,
        description: t`An icon on a muted tile.`,
    },
    image: {
        title: t.context("option")`Image`,
        description: t`A cropped picture.`,
    },
};

/** The properties people change in an item's controls and inspector. */
const ITEM_PROPERTIES = {
    variant: field
        .enum(ITEM_VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the item stands bare, outlined or on a muted fill.`)
        .default("default"),
    size: field
        .enum(ITEM_SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How much room the item leaves around its content.`)
        .default("default"),
};

/** The properties people change in an item's media's controls and inspector. */
const ITEM_MEDIA_PROPERTIES = {
    variant: field
        .enum(ITEM_MEDIA_VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`How the media is framed beside the item's content.`)
        .default("default"),
};

/** The styles of an item and its elements. */
const styles = style.create({
    item: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        borderWidth: stroke.border,
        borderColor: { default: "transparent", ":focus-visible": color.ring },
        borderRadius: radius[3],
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
        borderColor: { default: "transparent", [media.forcedColors]: "Canvas" },
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
            ":hover": {
                default: null,
                [media.hover]: color.hover,
            },
        },
    },
    outline: {
        backgroundColor: {
            default: "transparent",
            ":hover": {
                default: null,
                [media.hover]: color.hover,
            },
        },
    },
    muted: {
        backgroundColor: {
            default: `color-mix(in oklab, ${color.muted} 50%, transparent)`,
            ":hover": {
                default: null,
                [media.hover]: color.hover,
            },
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
export type ItemVariant = keyof typeof ITEM_VARIANTS;

/** The gap and padding of an item. */
export type ItemSize = keyof typeof ITEM_SIZES;

/** How an item frames its media: bare, as an icon on a muted tile, or as a cropped picture. */
export type ItemMediaVariant = keyof typeof ITEM_MEDIA_VARIANTS;

/** The properties of an element of an item, the native element's attributes included. */
export type ItemElementProperties<Attributes = JSX.HTMLAttributes<HTMLDivElement>> = Omit<
    Attributes,
    "class"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** The look of an item: its variant and size. */
type ItemLook = DeclaredProperties<typeof ITEM_PROPERTIES>;

/** Return the StyleX styles of an element that looks like an item, a link's included. */
function itemVariants(look: ItemLook): style.Styles {
    return [
        text.sm,
        state.focus,
        styles.item,
        variants[look.variant],
        links[look.variant],
        sizes[look.size],
    ];
}

/** Render a row of media, content and actions, listed as an entry inside a group. */
export const Item = Object.assign(
    defineComponent({
        name: "item",
        properties: ITEM_PROPERTIES,
        render(
            properties: RenderProperties<ItemElementProperties, typeof ITEM_PROPERTIES>,
        ): JSX.Element {
            // list the item in its group
            const isGrouped = useContext(ItemGroupContext);
            const rest = omit(properties, "variant", "size");

            return renderPart(
                "div",
                "item",
                rest,
                () => [
                    text.sm,
                    state.focus,
                    styles.item,
                    variants[properties.variant],
                    sizes[properties.size],
                ],
                {
                    get "data-variant"() {
                        return properties.variant;
                    },
                    get "data-size"() {
                        return properties.size;
                    },
                    get role() {
                        return isGrouped ? "listitem" : undefined;
                    },
                },
            );
        },
    }),
    {
        /** Read the styles of an item, for an element that only looks like one, such as a link. */
        styles: (look: Partial<ItemLook>): style.Styles =>
            itemVariants({ ...Item.defaults, ...look }),
    },
);

/** The properties of an item, the native element's attributes included. */
export type ItemProperties = ComponentProperties<typeof Item>;

/** Render items as a list. */
export function ItemGroup(properties: ItemElementProperties): JSX.Element {
    return (
        <ItemGroupContext value={true}>
            {renderPart("div", "item-group", properties, styles.group, { role: "list" })}
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
            xstyle={[styles.separator, properties.xstyle]}
        />
    );
}

/** Render an item's icon, avatar or picture. */
export const ItemMedia = defineComponent({
    name: "itemMedia",
    properties: ITEM_MEDIA_PROPERTIES,
    render(
        properties: RenderProperties<ItemElementProperties, typeof ITEM_MEDIA_PROPERTIES>,
    ): JSX.Element {
        const rest = omit(properties, "variant");

        return renderPart(
            "div",
            "item-media",
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

/** The properties of an item's media, the native element's attributes included. */
export type ItemMediaProperties = ComponentProperties<typeof ItemMedia>;

/** Render the title and description of an item, taking the row's free width. */
export function ItemContent(properties: ItemElementProperties): JSX.Element {
    return renderPart("div", "item-content", properties, styles.content);
}

/** Render the title of an item. */
export function ItemTitle(properties: ItemElementProperties): JSX.Element {
    return renderPart("div", "item-title", properties, styles.title);
}

/** Render the description under an item's title, cut to two lines. */
export function ItemDescription(
    properties: ItemElementProperties<JSX.HTMLAttributes<HTMLParagraphElement>>,
): JSX.Element {
    return renderPart("p", "item-description", properties, styles.description);
}

/** Render the buttons at the end of an item. */
export function ItemActions(properties: ItemElementProperties): JSX.Element {
    return renderPart("div", "item-actions", properties, styles.actions);
}

/** Render a full-width row above an item's media and content. */
export function ItemHeader(properties: ItemElementProperties): JSX.Element {
    return renderPart("div", "item-header", properties, styles.edge);
}

/** Render a full-width row below an item's media and content. */
export function ItemFooter(properties: ItemElementProperties): JSX.Element {
    return renderPart("div", "item-footer", properties, styles.edge);
}
