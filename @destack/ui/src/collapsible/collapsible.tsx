import * as style from "@destack/style";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";

/** The styles of a collapsible's trigger. */
const styles = style.create({
    trigger: {
        listStyle: "none",
        cursor: "pointer",
        "::-webkit-details-marker": { display: "none" },
    },
});

/** The properties of an element of a collapsible, the native element's attributes included. */
export type CollapsibleElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** Render a native disclosure that shows and hides its content. */
export function Collapsible(
    properties: CollapsibleElementProperties<JSX.DetailsHtmlAttributes<HTMLDetailsElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return <details data-slot="collapsible" {...rest} {...style.attrs(properties.style)} />;
}

/** Render the summary that toggles its collapsible. */
export function CollapsibleTrigger(
    properties: CollapsibleElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <summary
            data-slot="collapsible-trigger"
            {...rest}
            {...style.attrs(styles.trigger, properties.style)}
        />
    );
}

/** Render the content a collapsible shows when open. */
export function CollapsibleContent(
    properties: CollapsibleElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return <div data-slot="collapsible-content" {...rest} {...style.attrs(properties.style)} />;
}
