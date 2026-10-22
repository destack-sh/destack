/** A greeting for a name. */
export interface Greeting {
    /** The greeted name. */
    name: string;
    /** The greeting's text. */
    text: string;
}

/** Greet a name. */
export function greet(name: string): Greeting {
    return { name, text: `hello ${name}` };
}
