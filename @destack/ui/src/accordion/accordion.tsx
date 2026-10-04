import { Icon } from "@destack/icon";
import * as style from "@destack/style";
import { color, motion, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { createContext, createUniqueId, omit, useContext } from "solid-js";

/** The name the items of the nearest single accordion share, undefined for a multiple one. */
const AccordionContext = createContext<{ readonly name: string | undefined } | null>(null);

/** The styles of an accordion and its items. */
const styles = style.create({
    item: {
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        borderBottomColor: color.border,
        interpolateSize: "allow-keywords",
        "::details-content": {
            height: { default: 0, ":open": "auto" },
            overflow: "clip",
            transitionProperty: "height, content-visibility",
            transitionDuration: motion.durationMedium,
            transitionTimingFunction: motion.easingStandard,
            transitionBehavior: "allow-discrete",
        },
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
        cursor: "pointer",
        textDecoration: { default: "none", ":hover": "underline" },
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::-webkit-details-marker": { display: "none" },
    },
    chevron: {
        display: "inline-flex",
        flexShrink: 0,
        color: color.mutedForeground,
        transform: { default: "none", [style.when.ancestor(":open")]: "rotate(180deg)" },
        transitionProperty: "transform",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    content: {
        paddingBottom: space[4],
    },
});

/** Whether one or several items of an accordion open at a time. */
export type AccordionType = "single" | "multiple";

/** The properties of an accordion, the native element's attributes included. */
export interface AccordionProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** Whether opening an item closes the others, single by default. */
    readonly type?: AccordionType;
    /** The StyleX styles applied after the accordion's styles. */
    readonly style?: style.Styles;
}

/** The properties of an element of an accordion, the native element's attributes included. */
export type AccordionElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** Render a stack of disclosures, of which a single accordion keeps one open. */
export function Accordion(properties: AccordionProperties): JSX.Element {
    const rest = omit(properties, "type", "style");
    const name = properties.type === "multiple" ? undefined : createUniqueId();

    return (
        <AccordionContext value={{ name }}>
            <div data-slot="accordion" {...rest} {...style.attrs(properties.style)} />
        </AccordionContext>
    );
}

/** Render one native disclosure of the nearest accordion, named after it when single. */
export function AccordionItem(
    properties: AccordionElementProperties<JSX.DetailsHtmlAttributes<HTMLDetailsElement>>,
): JSX.Element {
    // read the accordion, refusing an item outside one
    const accordion = useContext(AccordionContext);
    if (accordion === null) {
        throw new TypeError("an accordion item needs an accordion around it");
    }
    const rest = omit(properties, "style");

    return (
        <details
            name={accordion.name}
            data-slot="accordion-item"
            {...rest}
            {...style.attrs(style.defaultMarker(), styles.item, properties.style)}
        />
    );
}

/** Render the summary that toggles its item, with a chevron that turns when open. */
export function AccordionTrigger(
    properties: AccordionElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const rest = omit(properties, "style", "children");

    return (
        <summary
            data-slot="accordion-trigger"
            {...rest}
            {...style.attrs(text.callout, styles.trigger, properties.style)}
        >
            {properties.children}
            <span {...style.attrs(styles.chevron)}>
                <Icon name="caret-down" />
            </span>
        </summary>
    );
}

/** Render the content an item reveals when open. */
export function AccordionContent(
    properties: AccordionElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="accordion-content"
            {...rest}
            {...style.attrs(text.footnote, styles.content, properties.style)}
        />
    );
}
