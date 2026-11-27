import { schema } from "@destack/schema";

/** The characters a glob pattern takes literally that a regular expression does not. */
const SPECIAL = /[.+^${}()|[\]\\]/gu;

/** Which modules of a package run code on import, as package.json's `sideEffects` declares them. */
export const SideEffects = Object.assign(
    schema.union([schema.boolean(), schema.array(schema.string().min(1))]),
    {
        /** Report whether a module at a path below its package runs code on import. */
        has(value: SideEffects, path: string): boolean {
            // take every module or none
            if (typeof value === "boolean") {
                return value;
            }

            // match the path against each glob, as webpack and Rollup match them
            const relative = path.replace(/^\.\//u, "");

            return value.some((pattern) => SideEffects.pattern(pattern).test(relative));
        },

        /** Read a glob as a regular expression, a pattern without a slash matching at any depth. */
        pattern(glob: string): RegExp {
            // match a pattern without a slash below any directory
            const relative = glob.replace(/^\.\//u, "");
            const anywhere = relative.includes("/") ? relative : `**/${relative}`;

            // read globstars, stars and question marks, and every other character as itself
            const source = anywhere
                .replaceAll(SPECIAL, String.raw`\$&`)
                .replaceAll("**/", "\u0000")
                .replaceAll("**", "\u0001")
                .replaceAll("*", "[^/]*")
                .replaceAll("?", "[^/]")
                .replaceAll("\u0000", "(?:.*/)?")
                .replaceAll("\u0001", ".*");

            return new RegExp(`^${source}$`, "u");
        },
    },
);

/** Which modules of a package run code on import. */
export type SideEffects = schema.Infer<typeof SideEffects>;
