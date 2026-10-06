import { attrs, type CompiledStyles, type InlineStyles, type StyleXArray } from "@stylexjs/stylex";

/** The styles an element takes, as StyleX merges them: compiled styles, markers, conditions and dynamic styles, nested in arrays. */
export type Styles = StyleXArray<
    null | undefined | boolean | CompiledStyles | Readonly<[CompiledStyles, InlineStyles]>
>;

/** The class and inline style an element takes from StyleX styles and a caller's own inline style. */
export interface StyleAttributes {
    /** The atomic class names. */
    readonly class?: string;
    /** The inline style: StyleX's dynamic values, then the caller's. */
    readonly style?: string;
}

/** A caller's inline style: CSS text, or an object of properties by their CSS names, as JSX `style` takes it. */
export type InlineStyle = string | object;

/** Read the attributes some StyleX styles give an element, with a caller's inline style applied after them. */
export function attributes(styles: Styles, inline?: InlineStyle | false | null): StyleAttributes {
    // read StyleX's class names and dynamic values
    const compiled = attrs(styles);
    const caller = textOf(inline);

    // append the caller's inline style to win on conflict
    const joined = [compiled.style, caller].filter((part) => part !== undefined && part !== "");

    return {
        ...(compiled.class === undefined ? {} : { class: compiled.class }),
        ...(joined.length === 0 ? {} : { style: joined.join(";") }),
    };
}

/** Write a caller's inline style as CSS text: its text, or its set properties by their CSS names. */
function textOf(inline: InlineStyle | false | null | undefined): string | undefined {
    if (inline === undefined || inline === null || inline === false) {
        return undefined;
    } else if (typeof inline === "string") {
        return inline;
    }

    return Object.entries(inline)
        .flatMap(([name, value]: [string, unknown]) =>
            typeof value === "string" || typeof value === "number" ? [`${name}:${value}`] : [],
        )
        .join(";");
}
