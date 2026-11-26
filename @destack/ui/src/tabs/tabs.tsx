import type { ObjectReference } from "@destack/sync";
import x from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createEffect,
    createMemo,
    type JSX,
    merge,
    omit,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";
import { Icon } from "../icon/index.ts";
import { createLayoutAnimation } from "../motion/index.ts";
import { OverflowContext, OverflowEntryContext, useOverflowEntry } from "../overflow/index.ts";
import {
    type ElementPartProperties,
    type PartEvent,
    type Render,
    renderPart,
} from "../part/index.ts";
import { Scroll, scrollFade } from "../position/index.ts";
import {
    TabsContext,
    TabsControl,
    TabsItemContext,
    type TabsItemControl,
    type TabsActivationMode,
    useTabs,
} from "./control.ts";
import { itemMarker } from "./tabs.stylex.ts";
import {
    type ComponentProperties,
    type RenderProperties,
    defineComponent,
} from "@destack/view/declare";
import { field } from "@destack/object";

/** The activation of tabs that set none. */
const DEFAULTS: Required<Pick<TabsAttributes, "activationMode">> = {
    activationMode: "automatic",
};

/** A tab that only its icon fits, its close button and its words left out. */
const NARROW_QUERY = "@container tab (max-width: 4rem)";

/** Select the item of a tab with the focus. */
const FOCUSED = ":has([role=tab]:focus-visible)";

/** Select what a drag shows of a tab. */
const PREVIEW = ":is([data-slot=drag-preview] > *)";

/** Whether the nearest tab narrows and cuts its title. */
const TitleContext = createContext<Accessor<boolean>>(() => false);

/** The look and the sizing of the nearest tab list. */
const TabsListContext = createContext<{
    /** The look. */
    readonly variant: Accessor<TabsVariant>;
    /** How wide an item's tab is. */
    readonly sizing: Accessor<TabsSizing>;
}>({ variant: () => "default", sizing: () => "fit" });

/** The tick after a tab, none beside a selected or hovered one. */
const TICK = {
    content: "''",
    position: "absolute",
    insetBlock: "25%",
    insetInlineEnd: 0,
    width: stroke.border,
    pointerEvents: "none",
    backgroundColor: color.border,
    opacity: {
        default: 1,
        ":last-child": 0,
        ":hover": 0,
        ":is([data-active])": 0,
        ":is([data-before-active])": 0,
        ":is([data-orientation=vertical])": 0,
        ":has(+ :hover)": 0,
    },
} as const;

/** The most width of a tab that is as wide as the others, 200 pixels, as page tabs have it. */
const FIXED_WIDTH = "12.5rem";

/** The least width of a tab that is as wide as the others, 36 pixels, which fits its icon. */
const ICON_WIDTH = "2.25rem";

/** The least width of a tab that shrinks, which fits its icon, its close button and a letter. */
const LEAST_WIDTH = "5rem";

/** The most width of a tab that shrinks, some twenty letters, as browsers draw them. */
const MOST_WIDTH = "14rem";

/** The styles of tabs and their elements. */
const styles = style.create({
    tabs: {
        display: "flex",
        gap: space[2],
        minWidth: 0,
        maxWidth: "100%",
    },
    // clip the tabs that scroll to the ground's corners
    list: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "safe center",
        width: "fit-content",
        maxWidth: "100%",
        overflow: "clip",
        color: color.mutedForeground,
    },
    content: {
        position: "relative",
        isolation: "isolate",
        display: "flex",
        flexGrow: 1,
        flexDirection: "inherit",
        alignItems: "inherit",
        justifyContent: "inherit",
        flexWrap: "inherit",
        minWidth: 0,
        minHeight: 0,
    },
    // show every tab where those that do not fit hide
    whole: {
        overflow: "visible",
        alignSelf: "stretch",
        flexShrink: 0,
        maxWidth: "none",
    },
    // narrow with the room, down to the tabs' least widths
    narrow: {
        flexShrink: 1,
        minWidth: "auto",
    },
    // take the room there is for tabs that share it
    sharing: {
        flexGrow: 1,
        width: "100%",
        justifyContent: "flex-start",
    },
    // narrow a tab from the width of its words, at most a few of them
    shrink: {
        flexGrow: 0,
        flexShrink: 1,
        flexBasis: "auto",
        display: "inline-grid",
        gridAutoFlow: "column",
        // ask for no more than the rest of it needs, and at most for its words too
        gridTemplateColumns: "minmax(0, 1fr)",
        gridAutoColumns: "auto",
        minWidth: LEAST_WIDTH,
        maxWidth: MOST_WIDTH,
        overflow: "hidden",
    },
    // share the room evenly, down to the icon alone
    fixed: {
        flexGrow: 1,
        flexBasis: ICON_WIDTH,
        minWidth: ICON_WIDTH,
        maxWidth: FIXED_WIDTH,
        containerType: "inline-size",
        containerName: "tab",
    },
    // narrow a tab without an item from the width of its words, as an item narrows
    alone: {
        flexGrow: 0,
        flexShrink: 1,
        flexBasis: "auto",
        minWidth: 0,
    },
    // take the width of a vertical list, whatever the words
    stretch: {
        display: "grid",
        gridAutoFlow: "column",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridAutoColumns: "auto",
        flexShrink: 0,
        minWidth: 0,
        containerType: "inline-size",
        containerName: "tab",
    },
    // show an icon alone in a narrow list
    icon: {
        // move the words out of view
        gap: { default: space[2], [NARROW_QUERY]: space[6] },
    },
    iconClose: {
        display: { default: "inline-flex", [NARROW_QUERY]: "none" },
    },
    // hide what does not fit, the room at the end the title's, which cuts its words
    cut: {
        justifyContent: "flex-start",
        minWidth: 0,
        paddingInlineEnd: 0,
        overflow: "hidden",
    },
    title: {
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    // cut the words softly where they end in less room, in their own direction
    titleCut: {
        overflow: "hidden",
        paddingInlineEnd: space[2],
        maskImage: {
            default: `linear-gradient(to right, black calc(100% - ${space[2]}), transparent)`,
            ":dir(rtl)": `linear-gradient(to left, black calc(100% - ${space[2]}), transparent)`,
            [media.forcedColors]: "none",
        },
    },
    item: {
        position: "relative",
        zIndex: 1,
        display: "inline-flex",
        alignItems: "center",
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        // ring the whole tab where its trigger has the focus
        outlineStyle: { default: "none", [FOCUSED]: "solid" },
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
        outlineColor: color.ring,
        // stand out from what a drag takes it over
        backgroundColor: { default: null, [PREVIEW]: color.card },
        boxShadow: { default: null, [PREVIEW]: shadow.raised },
        // be as wide as the tab that is dragged
        width: { default: null, [PREVIEW]: "100%" },
    },
    // fit inside its item's border
    inside: {
        height: `calc(${size[2]} - 2 * ${stroke.border})`,
        borderWidth: 0,
        outlineStyle: "none",
    },
    close: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
        width: `calc(${size[1]} - ${space[1]})`,
        height: `calc(${size[1]} - ${space[1]})`,
        marginInlineEnd: space[1],
        padding: 0,
        borderWidth: 0,
        borderRadius: radius[2],
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.hover },
        },
        color: "inherit",
        cursor: "pointer",
    },
    // put one in the place of the other
    closeFace: {
        display: "inline-flex",
        gridArea: "1 / 1",
    },
    closeFaces: {
        display: "inline-grid",
        placeItems: "center",
    },
    // show the cross under the pointer or the focus, and in the selected tab
    cross: {
        opacity: {
            default: 0,
            [style.when.ancestor(":is([data-active])", itemMarker)]: 1,
            [style.when.ancestor(":has(:focus-visible)", itemMarker)]: 1,
            [style.when.ancestor(":hover", itemMarker)]: { default: null, [media.hover]: 1 },
        },
    },
    // show what is there at rest until the pointer or the focus comes
    rest: {
        opacity: {
            default: 1,
            [style.when.ancestor(":has(:focus-visible)", itemMarker)]: 0,
            [style.when.ancestor(":hover", itemMarker)]: { default: null, [media.hover]: 0 },
        },
    },
    // show the cross in place of what is there at rest only then
    over: {
        opacity: {
            default: 0,
            [style.when.ancestor(":has(:focus-visible)", itemMarker)]: 1,
            [style.when.ancestor(":hover", itemMarker)]: { default: null, [media.hover]: 1 },
        },
    },
    // leave the room after it to what is beside it
    beside: {
        paddingInlineEnd: space[1],
    },
    indicator: {
        position: "absolute",
        top: 0,
        left: 0,
        borderRadius: radius[3],
        pointerEvents: "none",
        transitionProperty: "translate, width, height",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    place: (left: string, top: string, width: string, height: string) => ({
        translate: `${left} ${top}`,
        width,
        height,
    }),
    trigger: {
        position: "relative",
        zIndex: 1,
        display: "inline-flex",
        alignItems: "center",
        gap: space[2],
        height: size[2],
        paddingInline: space[3],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[3],
        fontWeight: weight.medium,
        color: { default: "inherit", ":hover": { default: null, [media.hover]: color.foreground } },
        whiteSpace: "nowrap",
        cursor: "pointer",
        transitionProperty: "color, background-color, box-shadow",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    selected: {
        forcedColorAdjust: { default: null, [media.forcedColors]: "none" },
        color: { default: color.foreground, [media.forcedColors]: "HighlightText" },
    },
    panel: {
        flex: 1,
    },
});

/** The ground of each variant's list. */
const variants = style.create({
    default: {
        borderRadius: radius[4],
        backgroundColor: color.muted,
    },
    line: {},
    card: {
        alignSelf: "stretch",
        alignItems: "flex-end",
        width: "auto",
        justifyContent: "flex-start",
        paddingBlockStart: space[1],
        paddingInline: space[1],
        backgroundColor: color.muted,
        boxShadow: `inset 0 calc(-1 * ${stroke.border}) 0 ${color.border}`,
    },
    subtle: {},
});

/** The room around each variant's tabs. */
const contents = style.create({
    default: {
        padding: stroke.ring,
    },
    line: {
        gap: space[1],
    },
    card: {},
    subtle: {
        gap: space[1],
    },
});

/** The look of each variant's tabs. */
const looks = style.create({
    default: {},
    line: {},
    // join the panel under them, a tick between those that are not selected
    card: {
        borderEndStartRadius: 0,
        borderEndEndRadius: 0,
        "::after": TICK,
    },
    subtle: {
        "::after": TICK,
        backgroundColor: {
            default: null,
            ":hover": { default: null, [media.hover]: color.hover },
        },
    },
});

/** The words of each variant's tabs. */
const words = style.create({
    default: {},
    line: {},
    card: {
        paddingInline: space[2],
        fontWeight: weight.regular,
    },
    subtle: {
        paddingInline: space[2],
        fontWeight: weight.regular,
    },
});

/** The mark of each variant's selected tab: a ground of its own, a line, or both. */
const marks = style.create({
    default: {
        backgroundColor: { default: color.card, [media.forcedColors]: "Highlight" },
        boxShadow: shadow.raised,
    },
    line: {
        backgroundColor: { default: null, [media.forcedColors]: "Highlight" },
        borderRadius: 0,
        boxShadow: {
            default: `inset 0 calc(-1 * ${stroke.ring}) 0 ${color.foreground}`,
            ":is([data-orientation=vertical])": `inset ${stroke.ring} 0 0 ${color.foreground}`,
            ":is([data-orientation=vertical]:dir(rtl))": `inset calc(-1 * ${stroke.ring}) 0 0 ${color.foreground}`,
        },
    },
    card: {
        backgroundColor: { default: color.background, [media.forcedColors]: "Highlight" },
        borderColor: color.border,
        borderBlockEndColor: color.background,
    },
    subtle: {
        backgroundColor: { default: color.card, [media.forcedColors]: "Highlight" },
        boxShadow: shadow.raised,
    },
});

/** The direction of each orientation's list above or beside the panels. */
const orientations = style.create({
    horizontal: { flexDirection: "column" },
    vertical: { flexDirection: "row", gap: space[4] },
});

/** The direction of each orientation's tabs within the list. */
const lists = style.create({
    horizontal: { flexDirection: "row" },
    vertical: {
        flexDirection: "column",
        alignItems: "stretch",
        alignSelf: "flex-start",
        justifyContent: "flex-start",
        maxHeight: "100%",
    },
});

/** The scrolling of tabs that do not fit, the sides with more past them faded. */
const scrolls = style.create({
    horizontal: {
        overflowX: "auto",
        scrollbarWidth: "none",
        scrollPaddingInline: space[6],
    },
    vertical: { overflowY: "auto", overscrollBehavior: "contain" },
});

/** The look of a tab list: on a ground, underlined, attached to the panel, or filled lightly. */
export type TabsVariant = keyof typeof variants;

/** How wide an item's tab is: its words, narrowing with the room, or as the others, narrowing. */
export type TabsSizing = "fit" | "shrink" | "fixed";

/** Return the StyleX styles of a variant's ground, such as for a row with more than its tabs. */
export function tabsListVariants(options: {
    /** The look, tabs on a muted ground by default. */
    readonly variant?: TabsVariant;
}): style.Styles {
    return variants[options.variant ?? "default"];
}

/** The room and alignment of each orientation's tabs: sharing the row, or set at their start. */
const triggers = style.create({
    horizontal: { flex: 1, justifyContent: "center" },
    vertical: { flexGrow: 1, flexShrink: 0, justifyContent: "flex-start" },
});

/** The properties of a tabs root, the native element's attributes included. */
export interface TabsAttributes extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "onClose"
> {
    /** The selected tab's value, which makes the selection controlled. */
    readonly value?: string;
    /** The tab selected at first when the selection is uncontrolled. */
    readonly defaultValue?: string;
    /** Handle another tab being selected. */
    readonly onValueChange?: (value: string) => void;
    /** Read the object a tab stands for, which actions on its type take, absent for none. */
    readonly reference?: ((value: string) => ObjectReference | undefined) | undefined;
    /** Handle a person moving a tab to a place among the others by an action. */
    readonly onMove?: (value: string, index: number) => void;
    /** Handle a person opening the tab closed last again. */
    readonly onReopen?: (() => void) | undefined;
    /** What the ids of the tabs and panels start with, one of their own by default. */
    readonly idPrefix?: string;
    /** Handle a person closing a tab, whose owner then removes it. */
    readonly onClose?: (value: string) => void;
    /** Whether focusing a tab selects it or Enter and Space do, automatic by default. */
    readonly activationMode?: TabsActivationMode;
    /** The StyleX styles applied after the root's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** The properties of an element of a set of tabs, the native element's attributes included. */
export type TabsElementProperties<Attributes> = Omit<Attributes, "class"> & ElementPartProperties;

/** The properties of a tab or a tab's panel. */
export type TabsValueProperties<Attributes> = TabsElementProperties<Omit<Attributes, "value">> & {
    /** The value the tab and its panel stand for. */
    readonly value: string;
};

/** The properties people change in tabs's controls and inspector. */
const TABS_PROPERTIES = {
    orientation: field
        .enum({
            horizontal: { title: t.context("option")`Horizontal` },
            vertical: { title: t.context("option")`Vertical` },
        })
        .labeled(t.context("label")`Orientation`)
        .described(t`Whether the tabs lie in a row or a column.`)
        .default("horizontal"),
};

/** Track the selected tab of a list of tabs and their panels. */
export const Tabs = defineComponent({
    name: "tabs",
    properties: TABS_PROPERTIES,
    render(properties: RenderProperties<TabsAttributes, typeof TABS_PROPERTIES>): JSX.Element {
        // share one set of tabs with the list, tabs and panels
        const tabs = merge(DEFAULTS, properties);
        const locale = useLocale();
        const rest = omit(
            tabs,
            "value",
            "defaultValue",
            "onValueChange",
            "onClose",
            "onMove",
            "reference",
            "onReopen",
            "idPrefix",
            "orientation",
            "activationMode",
        );
        const control = new TabsControl(tabs);

        return (
            <TabsContext value={control}>
                {renderPart(
                    "div",
                    "tabs",
                    rest,
                    () => [styles.tabs, orientations[tabs.orientation]],
                    {
                        get "data-orientation"() {
                            return tabs.orientation;
                        },
                        ref: TabsControl.handlers(
                            () => control,
                            () => locale.direction,
                            (value) => properties.reference?.(value),
                        ),
                    },
                )}
            </TabsContext>
        );
    },
});

/** The properties of a tab list, the native element's attributes included. */
export type TabsListAttributes = TabsElementProperties<JSX.HTMLAttributes<HTMLDivElement>>;

/** The properties people change in a tab list's controls and inspector. */
const TABS_LIST_PROPERTIES = {
    variant: field
        .enum({
            default: {
                title: t.context("option")`Default`,
                description: t`Tabs on a track, the shown one raised.`,
            },
            line: {
                title: t.context("option")`Line`,
                description: t`Tabs over a rule, the shown one underlined.`,
            },
            card: {
                title: t.context("option")`Card`,
                description: t`Tabs as the tops of cards, the shown one joined to its panel.`,
            },
            subtle: {
                title: t.context("option")`Subtle`,
                description: t`Quiet tabs, the shown one filled.`,
            },
        })
        .labeled(t.context("label")`Variant`)
        .described(t`How the tabs set the shown one apart.`)
        .default("default"),
    sizing: field
        .enum({
            fit: {
                title: t.context("option")`Fit`,
                description: t`Each tab as wide as its label.`,
            },
            shrink: {
                title: t.context("option")`Shrink`,
                description: t`Tabs narrow to fit their list, their labels cut short.`,
            },
            fixed: {
                title: t.context("option")`Fixed`,
                description: t`Every tab as wide as the others.`,
            },
        })
        .labeled(t.context("label")`Sizing`)
        .described(t`How wide each tab is.`)
        .default("fit"),
};

/** Render the tab list, which arrow keys, Home and End move through. */
export const TabsList = defineComponent({
    name: "tabsList",
    properties: TABS_LIST_PROPERTIES,
    render(
        properties: RenderProperties<TabsListAttributes, typeof TABS_LIST_PROPERTIES>,
    ): JSX.Element {
        // move the focus among the list's tabs, selecting them when activation is automatic
        const control = useTabs();
        const locale = useLocale();
        let element: HTMLElement | undefined;
        const move = (event: KeyboardEvent) => {
            // leave keys with modifiers to the actions
            if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
                return;
            }
            const target = control.list.focus.move(event, locale.direction);
            if (target !== undefined && control.properties.activationMode === "automatic") {
                control.select(target);
            }
        };

        // move the tabs to their places as they open, close and move
        createLayoutAnimation({
            elements: () =>
                control.list.items
                    .items()
                    .flatMap((tab) => control.element(tab.key) ?? tab.element() ?? []),
            around: () => element,
        });

        // bring the selected tab into view
        createEffect(
            () => {
                const value = control.value();

                return value === undefined ? undefined : control.element(value);
            },
            (selected) => {
                if (selected !== undefined && element !== undefined) {
                    Scroll.reveal(element, selected);
                }
            },
        );

        // show every tab in an overflow, else mark the sides it scrolls to
        const isWhole = useContext(OverflowContext) !== null;
        createEffect(
            () => control.list.items.items().length,
            () => (element === undefined || isWhole ? undefined : Scroll.follow(element)),
        );

        // share the look with the tabs
        const variant = (): TabsVariant => properties.variant;
        const sizing = (): TabsSizing => properties.sizing;

        // leave the ground to a row of more than tabs
        const isBare = (): boolean => isWhole && variant() === "card";

        return (
            <TabsListContext value={{ variant, sizing }}>
                {renderPart(
                    "div",
                    "tabs-list",
                    omit(properties, "variant", "sizing", "children"),
                    () => [
                        styles.list,
                        !isBare() && variants[variant()],
                        lists[control.properties.orientation],
                        isWhole && styles.whole,
                        sizing() !== "fit" && styles.narrow,
                        sizing() === "fixed" && styles.sharing,
                    ],
                    {
                        role: "tablist",
                        get children() {
                            return (
                                <div
                                    role="presentation"
                                    ref={(created) => (element = created)}
                                    {...style.attributes([
                                        styles.content,
                                        contents[variant()],
                                        !isWhole && [
                                            scrolls[control.properties.orientation],
                                            scrollFade.sides,
                                        ],
                                    ])}
                                >
                                    {properties.children}
                                </div>
                            );
                        },
                        get "data-variant"() {
                            return variant();
                        },
                        get "data-orientation"() {
                            return control.properties.orientation;
                        },
                        get "aria-orientation"() {
                            return control.properties.orientation;
                        },
                        onKeyDown: move,
                        onFocusIn: (event) => {
                            if (element !== undefined && event.target instanceof HTMLElement) {
                                Scroll.reveal(element, event.target);
                            }
                        },
                        onFocusOut: (event) => control.list.focus.focusOut(event),
                    },
                )}
            </TabsListContext>
        );
    },
});

/** The properties of a tab, the native button's attributes included. */
export type TabsTriggerAttributes = TabsElementProperties<
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "value" | "disabled">
> & {
    /** The value the tab and its panel stand for, its item's in an item. */
    readonly value?: string;
    /** Render another element with the tab's attributes, the native button by default. */
    readonly render?: Render;
};

/** The properties of a tab with what is beside it, the native element's attributes included. */
export type TabsItemProperties = TabsValueProperties<JSX.HTMLAttributes<HTMLDivElement>>;

/** Render a tab and what is beside it, such as its close button. */
export function TabsItem(properties: TabsItemProperties): JSX.Element {
    // wrap one tab
    const control = useTabs();
    const { variant, sizing } = useContext(TabsListContext);
    const value = untrack(() => properties.value);
    const closes = new CollectionBuilder<symbol>();
    let element: HTMLElement | undefined;
    const around = useContext(OverflowEntryContext);
    const entry = useOverflowEntry({ kind: "item", element: () => element }) ?? around;
    const item: TabsItemControl = {
        value,
        element: () => element,
        isClosable: () => closes.size() > 0,
        addClose: () => void closes.add(Symbol("close")),
        isHidden: () => entry?.isHidden() === true,
    };
    control.items.add(item);
    const isSelected = (): boolean => control.value() === value;
    const isBeforeSelected = createMemo(() => {
        // read whether the tab after it is selected
        const keys = control.list.items.items().map(({ key }) => key);

        return keys[keys.indexOf(value) + 1] === control.value();
    });

    return (
        <TabsItemContext value={item}>
            <OverflowEntryContext value={entry}>
                {renderPart(
                    "div",
                    "tabs-item",
                    omit(properties, "value"),
                    () => [
                        itemMarker,
                        styles.item,
                        looks[variant()],
                        sizing() !== "fit" && [
                            control.properties.orientation === "vertical"
                                ? styles.stretch
                                : [styles.shrink, sizing() === "fixed" && styles.fixed],
                        ],
                        isSelected() && [
                            styles.selected,
                            control.place() === undefined && marks[variant()],
                        ],
                    ],
                    {
                        role: "presentation",
                        "data-tab": value,
                        get "data-active"() {
                            return isSelected() ? "" : undefined;
                        },
                        get "data-before-active"() {
                            return isBeforeSelected() ? "" : undefined;
                        },
                        get "data-orientation"() {
                            return control.properties.orientation;
                        },
                        ref: (created) => {
                            element = created;
                        },
                        onMouseDown: (event) => {
                            // close by the middle button in place of the browser's scrolling
                            if (event.button === 1 && untrack(item.isClosable)) {
                                event.preventDefault();
                            }
                        },
                        onAuxClick: (event) => {
                            if (event.button === 1) {
                                control.close(value);
                            }
                        },
                    },
                )}
            </OverflowEntryContext>
        </TabsItemContext>
    );
}

/** Render the button that closes the tab of its item, its content what it shows at rest. */
export function TabsClose(
    properties: TabsElementProperties<JSX.ButtonHTMLAttributes<HTMLButtonElement>>,
): JSX.Element {
    // close the tab around it
    const control = useTabs();
    const locale = useLocale();
    const item = useContext(TabsItemContext);
    if (item === null) {
        throw new TypeError("a tab's close button needs a tab's item around it");
    }
    item.addClose();
    const rest = omit(properties, "children");

    return renderPart(
        "button",
        "tabs-close",
        rest,
        [state.focusInset, state.touch, styles.close, styles.iconClose],
        {
            type: "button" as const,
            tabindex: -1,
            get "aria-label"() {
                return locale.render(t.context("action")`Close`);
            },
            get "aria-describedby"() {
                return control.tabId(item.value);
            },
            onClick: () => control.close(item.value),
            get children() {
                // show the cross in place of what is there at rest
                return "children" in properties ? (
                    <span {...style.attributes([styles.closeFaces])}>
                        <span {...style.attributes([styles.closeFace, styles.rest])}>
                            {properties.children}
                        </span>
                        <span {...style.attributes([styles.closeFace, styles.over])}>
                            <Icon icon={x} size="0.75em" />
                        </span>
                    </span>
                ) : (
                    <span {...style.attributes([styles.closeFace, styles.cross])}>
                        <Icon icon={x} size="0.75em" />
                    </span>
                );
            },
        },
    );
}

/** Render what slides to the selected tab, in place of the selected tab's own mark. */
export function TabsIndicator(
    properties: TabsElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    // follow the selected tab, its list and its size
    const control = useTabs();
    const { variant } = useContext(TabsListContext);
    createEffect(
        () => {
            const value = control.value();

            return value === undefined ? undefined : control.element(value);
        },
        (element) => {
            // hide without a selected tab
            if (element === undefined) {
                return undefined;
            }
            const measure = (): void =>
                control.setPlace({
                    left: element.offsetLeft,
                    top: element.offsetTop,
                    width: element.offsetWidth,
                    height: element.offsetHeight,
                });
            measure();

            // follow every tab, as each moves the ones after it
            const resizes = new ResizeObserver(measure);
            const list = element.parentElement;
            [element, ...(list === null ? [] : [list, ...list.children])].forEach((each) =>
                resizes.observe(each),
            );

            return () => {
                resizes.disconnect();
                control.setPlace(undefined);
            };
        },
    );

    return renderPart(
        "span",
        "tabs-indicator",
        properties,
        () => {
            const place = control.place();

            return [
                styles.indicator,
                marks[variant()],
                place !== undefined &&
                    styles.place(
                        `${String(place.left)}px`,
                        `${String(place.top)}px`,
                        `${String(place.width)}px`,
                        `${String(place.height)}px`,
                    ),
            ];
        },
        {
            "aria-hidden": "true",
            get "data-orientation"() {
                return control.properties.orientation;
            },
            get hidden() {
                return control.place() === undefined;
            },
        },
    );
}

/** The properties people change in a tab's controls and inspector. */
const TABS_TRIGGER_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses the focus and presses where the tab cannot show now.`)
        .default(false),
};

/** Render a tab that selects its panel on click, or on focus when activation is automatic. */
export const TabsTrigger = defineComponent({
    name: "tabsTrigger",
    properties: TABS_TRIGGER_PROPERTIES,
    render(
        properties: RenderProperties<TabsTriggerAttributes, typeof TABS_TRIGGER_PROPERTIES>,
    ): JSX.Element {
        // take its own value, or its item's
        const control = useTabs();
        const { variant, sizing } = useContext(TabsListContext);
        const item = useContext(TabsItemContext);
        const value = untrack(() => properties.value) ?? item?.value;
        if (value === undefined) {
            throw new TypeError("a tab is invalid: it has neither a value nor an item around it");
        }

        // join the tabs and read whether this one is selected
        let element: HTMLElement | undefined;
        const entry = useOverflowEntry({ kind: "item", element: () => element });
        control.list.add({
            key: value,
            text: () => value,
            isDisabled: () =>
                properties.disabled || entry?.isHidden() === true || item?.isHidden() === true,
            element: () => element,
        });
        const rest = omit(properties, "value");
        const isSelected = (): boolean => control.value() === value;
        const isCut = (): boolean => sizing() !== "fit";

        return (
            <TitleContext value={isCut}>
                {renderPart(
                    "button",
                    "tabs-trigger",
                    rest,
                    () => [
                        text.sm,
                        state.focusInset,
                        state.disabled,
                        styles.trigger,
                        triggers[control.properties.orientation],
                        item === null ? looks[variant()] : styles.inside,
                        words[variant()],
                        item?.isClosable() === true && styles.beside,
                        item === null &&
                            isCut() &&
                            control.properties.orientation === "horizontal" && [
                                styles.alone,
                                sizing() === "fixed" && styles.fixed,
                            ],
                        isCut() &&
                            (control.properties.orientation === "vertical" || sizing() === "fixed"
                                ? [styles.cut, styles.icon]
                                : styles.cut),
                        isSelected() && [
                            item === null && styles.selected,
                            item === null && control.place() === undefined && marks[variant()],
                        ],
                    ],
                    {
                        type: "button" as const,
                        role: "tab" as const,
                        get id() {
                            return control.tabId(value);
                        },
                        get "aria-controls"() {
                            return control.panelId(value);
                        },
                        get "aria-selected"() {
                            return isSelected() ? "true" : "false";
                        },
                        get "aria-keyshortcuts"() {
                            return item?.isClosable() === true ? "Delete" : undefined;
                        },
                        get tabindex() {
                            return control.list.focus.isActive(value) ? 0 : -1;
                        },
                        ref: (target: HTMLElement) => {
                            element = target;
                        },
                        get "data-active"() {
                            return isSelected() ? "" : undefined;
                        },
                        get "data-value"() {
                            return value;
                        },
                        get "data-orientation"() {
                            return control.properties.orientation;
                        },
                        onClick: () => control.select(value),
                        onFocus: () => control.list.focus.focusIn(value),
                        onKeyDown: (event: PartEvent<KeyboardEvent>) => {
                            if (event.key === "Delete") {
                                control.close(value);
                            }
                        },
                    },
                )}
            </TitleContext>
        );
    },
});

/** Render the words of a tab, cut in their own direction where the tab narrows. */
export function TabsTitle(
    properties: TabsElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const isCut = useContext(TitleContext);

    return renderPart(
        "span",
        "tabs-title",
        properties,
        () => [styles.title, isCut() && styles.titleCut],
        {
            dir: "auto",
        },
    );
}

/** Render the panel of a tab, hidden while another tab is selected. */
export function TabsContent(
    properties: TabsValueProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useTabs();
    const rest = omit(properties, "value");

    return renderPart("div", "tabs-content", rest, [state.focus, styles.panel], {
        role: "tabpanel" as const,
        get id() {
            return control.panelId(properties.value);
        },
        get "aria-labelledby"() {
            return control.tabId(properties.value);
        },
        tabindex: 0,
        get hidden() {
            return control.value() !== properties.value;
        },
        get "data-hidden"() {
            return control.value() === properties.value ? undefined : "";
        },
        get "data-orientation"() {
            return control.properties.orientation;
        },
    });
}

/** The properties of tabs, its native attributes included. */
export type TabsProperties = ComponentProperties<typeof Tabs>;

/** The properties of a tab list, its native attributes included. */
export type TabsListProperties = ComponentProperties<typeof TabsList>;

/** The properties of a tab, its native attributes included. */
export type TabsTriggerProperties = ComponentProperties<typeof TabsTrigger>;
