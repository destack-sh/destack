import { field } from "@destack/object";
import { t } from "@destack/locale";
import { state } from "@destack/theme/state";
import { ORIENTATION } from "../position/index.ts";
import { Icon } from "@destack/ui/icon";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    omit,
    useContext,
    useLocale,
} from "@destack/view";
import { SELECTION_OPTIONS, Selection, type SelectionProperties } from "../selection/index.ts";
import { disclosure, DisclosurePanel, followToggle, refuseDisabled } from "../disclosure/index.ts";
import { ListState } from "../focus/index.ts";
import { renderPart, type Render } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The open items of the nearest accordion, null outside one. */
const AccordionContext = createContext<AccordionControl | null>(null);

/** Read the nearest accordion, refusing a part outside one. */
export function useAccordion(): AccordionControl {
    const accordion = useContext(AccordionContext);
    if (accordion === null) {
        throw new TypeError("an accordion part needs an accordion around it");
    }

    return accordion;
}

/** The item of the nearest accordion item, null outside one. */
const AccordionItemContext = createContext<AccordionItemControl | null>(null);

/** Read the nearest accordion item, refusing a part outside one. */
export function useAccordionItem(): AccordionItemControl {
    const item = useContext(AccordionItemContext);
    if (item === null) {
        throw new TypeError("an accordion part needs an accordion item around it");
    }

    return item;
}

/** The styles of an accordion and its items. */
const styles = style.create({
    item: {
        borderBottomStyle: "solid",
        borderBottomWidth: { default: stroke.border, ":last-child": 0 },
        borderBottomColor: color.border,
    },
    trigger: {
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "space-between",
        gap: space[4],
        paddingBlock: space[4],
        borderRadius: radius[3],
        fontWeight: weight.medium,
        listStyle: "none",
        cursor: { default: "pointer", ":is([aria-disabled=true])": "not-allowed" },
        opacity: { default: 1, ":is([aria-disabled=true])": 0.5 },
        textDecoration: {
            default: "none",
            ":hover": { default: null, [media.hover]: "underline" },
        },
        "::-webkit-details-marker": { display: "none" },
    },
    chevron: {
        display: "inline-flex",
        alignItems: "center",
        flexShrink: 0,
        // center on the title's first line
        height: "1lh",
        color: color.mutedForeground,
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    turned: { transform: "rotate(180deg)" },
    content: {
        paddingBottom: space[4],
    },
});

/** The properties people change in an accordion's controls and inspector. */
const ACCORDION_PROPERTIES = {
    orientation: ORIENTATION.described(
        t`Whether the sections stack in a column or lie in a row.`,
    ).default("vertical"),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses presses on every section of the accordion.`)
        .default(false),
};

/** The properties of an accordion besides its declared ones: its open sections and attributes. */
export type AccordionAttributes = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> & {
    /** The StyleX styles applied after the accordion's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
} & SelectionProperties;

/** The properties people change in an accordion section's controls and inspector. */
const ACCORDION_ITEM_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses presses on this section alone.`)
        .default(false),
};

/** The properties of an accordion section besides its declared ones: its value and attributes. */
export type AccordionItemAttributes = AccordionElementProperties<
    Omit<JSX.DetailsHtmlAttributes<HTMLDetailsElement>, "open" | "name">
> & {
    /** The value the item stands for in the accordion's open values. */
    readonly value: string;
};

/** The properties of an element of an accordion, the native element's attributes included. */
export type AccordionElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** The open items of an accordion and its triggers, which its items share. */
export class AccordionControl {
    /** The properties of the accordion root, read for its controlled state. */
    readonly properties: RenderProperties<AccordionAttributes, typeof ACCORDION_PROPERTIES>;
    /** The name a single accordion's items share, so the platform has one open at a time. */
    readonly name: string | undefined;
    /** The open values, controlled or the accordion's own. */
    readonly selection: Selection;
    /** The triggers in document order, which the arrow keys move between. */
    readonly list: ListState;

    /** Create the state of an accordion from its root's properties. */
    constructor(properties: RenderProperties<AccordionAttributes, typeof ACCORDION_PROPERTIES>) {
        // follow the controlled open values or the accordion's own
        this.properties = properties;
        this.name = properties.multiple === true ? undefined : createUniqueId();
        this.selection = new Selection(properties);
        this.list = new ListState({
            get orientation() {
                return properties.orientation;
            },
            isLooping: true,
            isTypeahead: false,
        });
    }

    /** Report whether an item is open. */
    isOpen(value: string): boolean {
        return this.selection.isSelected(value);
    }

    /** Open or close an item as its disclosure toggled. */
    toggle(value: string, isOpening: boolean, element: HTMLDetailsElement): void {
        // leave the state alone when a single accordion closes an item for another
        const isReplaced =
            this.name !== undefined &&
            element.parentElement?.querySelector(`details[name="${this.name}"][open]`) != null;
        if (!isOpening && isReplaced) {
            return;
        }

        // open or close the item
        if (isOpening !== this.isOpen(value)) {
            this.selection.toggle(value);
        }
    }
}

/** The value and availability of an accordion item, which its trigger reads. */
export interface AccordionItemControl {
    /** The value the item stands for. */
    readonly value: string;
    /** Whether the item's trigger ignores the person. */
    readonly isDisabled: Accessor<boolean>;
    /** Whether the item is open. */
    readonly isOpen: Accessor<boolean>;
}

/** Render a stack of disclosures that arrow keys move between. */
export const Accordion = defineComponent({
    name: "accordion",
    properties: ACCORDION_PROPERTIES,
    render(
        properties: RenderProperties<AccordionAttributes, typeof ACCORDION_PROPERTIES>,
    ): JSX.Element {
        // share the open items with the items and read the text direction for the arrow keys
        const control = new AccordionControl(properties);
        const locale = useLocale();
        const rest = omit(properties, ...SELECTION_OPTIONS, "orientation", "disabled");

        return (
            <AccordionContext value={control}>
                {renderPart("div", "accordion", rest, null, {
                    get "data-orientation"() {
                        return properties.orientation;
                    },
                    onKeyDown: (event) => {
                        // move between the triggers when the focus rests on one
                        if (
                            event.target instanceof HTMLElement &&
                            event.target.dataset["slot"] === "accordion-trigger"
                        ) {
                            control.list.focus.move(event, locale.direction);
                        }
                    },
                    onFocusOut: (event) => control.list.focus.focusOut(event),
                })}
            </AccordionContext>
        );
    },
});

/** The properties of an accordion, its native attributes included. */
export type AccordionProperties = ComponentProperties<typeof Accordion>;

/** Render one native disclosure of the nearest accordion, open while its value is. */
export const AccordionItem = defineComponent({
    name: "accordionItem",
    properties: ACCORDION_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<AccordionItemAttributes, typeof ACCORDION_ITEM_PROPERTIES>,
    ): JSX.Element {
        // read the accordion, refusing an item outside one
        const accordion = useAccordion();
        const rest = omit(properties, "value", "disabled");
        const isDisabled = (): boolean => properties.disabled || accordion.properties.disabled;

        return (
            <AccordionItemContext
                value={{
                    value: properties.value,
                    isDisabled,
                    isOpen: () => accordion.isOpen(properties.value),
                }}
            >
                {renderPart("details", "accordion-item", rest, [disclosure.root, styles.item], {
                    get name() {
                        return accordion.name;
                    },
                    get "data-open"() {
                        return accordion.isOpen(properties.value) ? "" : undefined;
                    },
                    get "data-closed"() {
                        return accordion.isOpen(properties.value) ? undefined : "";
                    },
                    get "data-disabled"() {
                        return isDisabled() ? "" : undefined;
                    },
                    get open() {
                        return accordion.isOpen(properties.value);
                    },
                    onToggle: (event) =>
                        followToggle(
                            event,
                            () => accordion.isOpen(properties.value),
                            (isOpening) =>
                                accordion.toggle(properties.value, isOpening, event.currentTarget),
                            "value" in accordion.properties,
                        ),
                })}
            </AccordionItemContext>
        );
    },
});

/** The properties of an accordion section, its native attributes included. */
export type AccordionItemProperties = ComponentProperties<typeof AccordionItem>;

/** Render the summary that toggles its item, with a chevron that turns when open. */
export function AccordionTrigger(
    properties: AccordionElementProperties<
        Omit<JSX.HTMLAttributes<HTMLElement>, "ref" | "onFocus">
    >,
): JSX.Element {
    // read the item, refusing a trigger outside one, and join the accordion's triggers
    const accordion = useAccordion();
    const item = useAccordionItem();
    const isDisabled = (): boolean => item.isDisabled();
    const rest = omit(properties, "children");
    let element: HTMLElement | undefined;
    accordion.list.add({
        key: item.value,
        text: () => item.value,
        isDisabled,
        element: () => element,
    });

    return renderPart(
        "summary",
        "accordion-trigger",
        rest,
        [text.sm, state.focus, styles.trigger],
        {
            get "data-panel-open"() {
                return item.isOpen() ? "" : undefined;
            },
            get "data-disabled"() {
                return isDisabled() ? "" : undefined;
            },
            get "aria-disabled"() {
                return isDisabled() ? "true" : undefined;
            },
            ref: (summary) => {
                element = summary;
                refuseDisabled(summary, isDisabled);
            },
            onFocus: () => accordion.list.focus.focusIn(item.value),
            get children() {
                return (
                    <>
                        {properties.children}
                        <span {...style.attrs(styles.chevron, item.isOpen() && styles.turned)}>
                            <Icon name="caret-down" />
                        </span>
                    </>
                );
            },
        },
    );
}
/** Render the content an item reveals when open. */
export function AccordionContent(
    properties: AccordionElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const item = useAccordionItem();

    return (
        <DisclosurePanel isOpen={() => item.isOpen()}>
            {renderPart("div", "accordion-content", properties, [text.sm, styles.content], {
                get "data-open"() {
                    return item.isOpen() ? "" : undefined;
                },
                get "data-closed"() {
                    return item.isOpen() ? undefined : "";
                },
            })}
        </DisclosurePanel>
    );
}
