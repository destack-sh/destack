import type { Reply } from "./reply.ts";

/** The RFC 5321 service extensions a server lists in its EHLO reply. */
export class Extensions {
    /** Each listed keyword in upper case, with its parameters in upper case. */
    readonly keywords: ReadonlyMap<string, readonly string[]>;

    /** Create a listing of extensions. */
    constructor(keywords: ReadonlyMap<string, readonly string[]>) {
        this.keywords = keywords;
    }

    /** Parse the extensions from an EHLO reply's lines after the first. */
    static parse(reply: Reply): Extensions {
        // split each line after the greeting into its keyword and parameters
        const lines = reply.text.split("\n").slice(1);
        const keywords = lines.map((line) => {
            // split at the first space
            const upper = line.toUpperCase();
            const space = upper.indexOf(" ");
            const keyword = space === -1 ? upper : upper.slice(0, space);
            const parameters = space === -1 ? [] : upper.slice(space + 1).split(" ");

            return [keyword, parameters] as const;
        });

        return new Extensions(new Map(keywords));
    }

    /** Report whether the server lists a keyword together with each given parameter. */
    has(keyword: string, ...parameters: readonly string[]): boolean {
        const listed = this.keywords.get(keyword);

        return listed !== undefined && parameters.every((parameter) => listed.includes(parameter));
    }
}
