import { t } from "@destack/locale";
import { field } from "@destack/object";
import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createUniqueId,
    type JSX,
    omit,
    useContext,
} from "@destack/view";
import {
    disclosure,
    DisclosureCaret,
    DisclosurePanel,
    followToggle,
    indicator,
    refuseDisabled,
} from "../disclosure/index.ts";
import { OpenChange, type OpenProperties, OpenState } from "../layer/index.ts";
import { contentOr, type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The nearest collapsible, null outside one. */
const CollapsibleContext = createContext<CollapsibleControl | null>(null);

/** Read the nearest collapsible, refusing a part outside one. */
export function useCollapsible(): CollapsibleControl {
    const control = useContext(CollapsibleContext);
    if (control === null) {
        throw new TypeError("a collapsible part needs a collapsible around it");
    }

    return control;
}

/** The open state and availability of a collapsible, which its trigger and content read. */
export interface CollapsibleControl {
    /** Whether the content shows. */
    readonly isOpen: Accessor<boolean>;
    /** Whether the trigger ignores the person. */
    readonly isDisabled: Accessor<boolean>;
}

/** The styles of a collapsible and its parts. */
const styles = style.create({
    trigger: {
        listStyle: "none",
        cursor: { default: "pointer", ":is([aria-disabled=true])": "not-allowed" },
        "::-webkit-details-marker": { display: "none" },
    },
    // run a line down the start of it, below the middle of the trigger's icon
    line: {
        minWidth: 0,
        marginBlockStart: space[2],
        marginInlineStart: `calc((1em - ${stroke.border}) / 2)`,
        paddingInlineStart: space[4],
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
    },
});

/** The properties of an element of a collapsible, the native element's attributes included. */
export type CollapsibleElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** The properties people change in a collapsible's controls and inspector. */
const COLLAPSIBLE_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses presses on its trigger.`)
        .default(false),
};

/** The properties of a collapsible besides its declared ones: its open state and attributes. */
export type CollapsibleAttributes = CollapsibleElementProperties<
    Omit<JSX.DetailsHtmlAttributes<HTMLDetailsElement>, "open">
> &
    OpenProperties & {};

/** Render a native disclosure that shows and hides its content. */
export const Collapsible = defineComponent({
    name: "collapsible",
    properties: COLLAPSIBLE_PROPERTIES,
    render(
        properties: RenderProperties<CollapsibleAttributes, typeof COLLAPSIBLE_PROPERTIES>,
    ): JSX.Element {
        // follow the controlled state or the collapsible's own
        const rest = omit(properties, "open", "defaultOpen", "onOpenChange", "disabled");
        const collapsible = new OpenState(properties, createUniqueId());
        const { isOpen } = collapsible;

        return (
            <CollapsibleContext value={{ isOpen, isDisabled: () => properties.disabled }}>
                {renderPart("details", "collapsible", rest, disclosure.root, {
                    get "data-open"() {
                        return isOpen() ? "" : undefined;
                    },
                    get "data-closed"() {
                        return isOpen() ? undefined : "";
                    },
                    get "data-disabled"() {
                        return properties.disabled ? "" : undefined;
                    },
                    get open() {
                        return isOpen();
                    },
                    onToggle: (event) =>
                        followToggle(
                            event,
                            isOpen,
                            (open) =>
                                collapsible.follow(open, new OpenChange("trigger-press", event)),
                            true,
                        ),
                })}
            </CollapsibleContext>
        );
    },
});

/** The properties of a collapsible, its native attributes included. */
export type CollapsibleProperties = ComponentProperties<typeof Collapsible>;

/** Render the summary that toggles its collapsible. */
export function CollapsibleTrigger(
    properties: CollapsibleElementProperties<Omit<JSX.HTMLAttributes<HTMLElement>, "ref">>,
): JSX.Element {
    // read the collapsible's state and availability
    const control = useCollapsible();
    const isDisabled = (): boolean => control.isDisabled();

    return renderPart("summary", "collapsible-trigger", properties, styles.trigger, {
        get "data-panel-open"() {
            return control.isOpen() ? "" : undefined;
        },
        get "data-disabled"() {
            return isDisabled() ? "" : undefined;
        },
        get "aria-disabled"() {
            return isDisabled() ? "true" : undefined;
        },
        ref: (element) => refuseDisabled(element, isDisabled),
    });
}

/** Render the mark of a trigger that turns as its collapsible opens, a caret by default. */
export function CollapsibleIndicator(
    properties: CollapsibleElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    // turn down from the reading direction
    const control = useCollapsible();
    const rest = omit(properties, "children");

    return renderPart(
        "span",
        "collapsible-indicator",
        rest,
        () => [indicator.base, control.isOpen() && indicator.open],
        {
            "aria-hidden": "true" as const,
            get children() {
                return contentOr(properties, () => <DisclosureCaret />);
            },
        },
    );
}

/** The properties people change in a collapsible's content's controls and inspector. */
const COLLAPSIBLE_CONTENT_PROPERTIES = {
    variant: field
        .enum({
            default: { title: t.context("option")`Default` },
            line: {
                title: t.context("option")`Line`,
                description: t`Indented beside a line, as nested items are.`,
            },
        })
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the content sits beside a line.`)
        .default("default"),
};

/** The properties of a collapsible's content besides its declared ones: its attributes. */
export type CollapsibleContentAttributes = CollapsibleElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> & {};

/** Render the content a collapsible shows when open. */
export const CollapsibleContent = defineComponent({
    name: "collapsibleContent",
    properties: COLLAPSIBLE_CONTENT_PROPERTIES,
    render(
        properties: RenderProperties<
            CollapsibleContentAttributes,
            typeof COLLAPSIBLE_CONTENT_PROPERTIES
        >,
    ): JSX.Element {
        // lay it out along a line when asked
        const control = useCollapsible();
        const rest = omit(properties, "variant");
        const layout = (): style.Styles => (properties.variant === "line" ? styles.line : null);

        return (
            <DisclosurePanel isOpen={() => control.isOpen()}>
                {renderPart("div", "collapsible-content", rest, layout, {
                    get "data-open"() {
                        return control.isOpen() ? "" : undefined;
                    },
                    get "data-closed"() {
                        return control.isOpen() ? undefined : "";
                    },
                })}
            </DisclosurePanel>
        );
    },
});

/** The properties of a collapsible's content, its native attributes included. */
export type CollapsibleContentProperties = ComponentProperties<typeof CollapsibleContent>;
