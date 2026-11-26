/** A pattern a JSON schema declares, matched by the browser's own engine. */
export class Pattern extends RegExp {
    /** Compile a pattern as a Unicode regular expression. */
    constructor(pattern: string) {
        try {
            super(pattern, "u");
        } catch (error) {
            throw new TypeError(`json schema pattern '${pattern}' is not declarable`, {
                cause: error,
            });
        }
    }
}
