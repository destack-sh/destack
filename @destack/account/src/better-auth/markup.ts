import { nth, present } from "@destack/schema";

/** The HTML escapes of the characters markup gives meaning to. */
const ESCAPES: Readonly<Record<string, string>> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
};

/** HTML a mail wrote or escaped. */
export class Markup {
    /** The HTML text. */
    readonly text: string;

    /** Wrap HTML text. */
    constructor(text: string) {
        this.text = text;
    }
}

/** Build markup, and escape every interpolated text value. */
export function html(
    strings: TemplateStringsArray,
    ...values: readonly (string | Markup | readonly Markup[])[]
): Markup {
    // join the strings with the written values
    let text = nth(strings, 0);
    for (const [index, value] of values.entries()) {
        text += write(value) + nth(strings, index + 1);
    }

    return new Markup(text);
}

/** Write an interpolated value: markup as it is, text escaped, a list of markup joined. */
function write(value: string | Markup | readonly Markup[]): string {
    // pass markup through
    if (value instanceof Markup) {
        return value.text;
    }
    // escape text
    else if (typeof value === "string") {
        return escape(value);
    }
    // join a list of markup
    else {
        return value.map((item) => item.text).join("");
    }
}

/** Escape a value for HTML text and attributes. */
function escape(value: string): string {
    return value.replace(/[&<>"']/gu, (character) =>
        present(ESCAPES[character], `the escape of ${character}`),
    );
}
