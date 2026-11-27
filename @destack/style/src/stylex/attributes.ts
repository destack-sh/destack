import { props, type CompiledStyles, type InlineStyles, type StyleXArray } from "@stylexjs/stylex";

/** The styles an element takes, as StyleX merges them. */
export type Styles = StyleXArray<
    null | undefined | boolean | CompiledStyles | Readonly<[CompiledStyles, InlineStyles]>
>;

/** The class and inline style an element takes from StyleX styles and a caller's inline style. */
export interface StyleAttributes {
    /** The atomic class names. */
    readonly class?: string;
    /** The inline style: StyleX's dynamic values, then the caller's. */
    readonly style?: string;
}

/** An inline style as JSX `style` takes it: CSS text, or an object of properties by CSS name. */
export type InlineStyle = string | object;

/** Read the class names and dynamic values some StyleX styles give an element. */
export function attrs(...styles: Styles[]): StyleAttributes {
    return attributes(styles);
}

/** Read the attributes StyleX styles give an element, then apply a caller's inline style. */
export function attributes(styles: Styles, inline?: InlineStyle | false | null): StyleAttributes {
    // read StyleX's class names and the custom properties its dynamic values set
    const compiled = props(styles);

    // append the caller's inline style to win on conflict
    const parts = [cssText(compiled.style), cssText(inline)].filter((part) => part !== "");

    return {
        ...(compiled.className === undefined ? {} : { class: compiled.className }),
        ...(parts.length === 0 ? {} : { style: parts.join(";") }),
    };
}

/** Write an inline style as CSS text, with every property name as written. */
function cssText(inline: InlineStyle | false | null | undefined): string {
    // no style
    if (inline === undefined || inline === null || inline === false) {
        return "";
    }
    // CSS text as written
    else if (typeof inline === "string") {
        return inline;
    }

    // properties, skipping the ones a caller left unset
    return Object.entries(inline)
        .flatMap(([name, value]: [string, unknown]) =>
            typeof value === "string" || typeof value === "number" ? [`${name}:${value}`] : [],
        )
        .join(";");
}
