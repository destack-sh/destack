import { RE2JS } from "re2js";

/** A pattern a JSON schema declares, matched in time linear in its input, as RE2 matches. */
export class Pattern extends RegExp {
    /** The pattern compiled for RE2's linear-time matching. */
    readonly #linear: RE2JS;

    /** Compile a pattern for RE2, refusing the lookarounds and backreferences RE2 lacks. */
    constructor(pattern: string) {
        super(pattern, "u");
        try {
            this.#linear = RE2JS.compile(pattern);
        } catch (error) {
            throw new TypeError(`json schema pattern '${pattern}' is not declarable`, {
                cause: error,
            });
        }
    }

    /** Find the pattern in a text, in time linear in the text. */
    override test(text: string): boolean {
        return this.#linear.matcher(text).find();
    }

    /** Refuse matching by position, which only `test` needs. */
    override exec(): never {
        throw new TypeError("a linear pattern supports only test");
    }
}
