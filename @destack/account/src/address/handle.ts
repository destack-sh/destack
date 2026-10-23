import { schema } from "@destack/schema";
import { Label } from "./label.ts";

/** The longest handle, sized with `SPACE_NAME_LENGTH` so an address fits one DNS label. */
export const HANDLE_LENGTH = 24;

/** The handle suggested when a name yields no handle characters. */
const DEFAULT_HANDLE_STEM = "user";

/** An account's handle: a label of at most `HANDLE_LENGTH` characters. */
export const AccountHandle = Object.assign(Label.schema(HANDLE_LENGTH), {
    /** Make a name, such as a login or an email's local part, the stem of suggested handles. */
    stem(name: string): string {
        return (
            name
                .toLowerCase()
                .replace(/[^a-z0-9]+/gu, "-")
                .slice(0, HANDLE_LENGTH)
                .replace(/^-+|-+$/gu, "") || DEFAULT_HANDLE_STEM
        );
    },

    /** List the suggested handles of a stem from a number on: the stem, then the stem numbered. */
    candidates(stem: string, first: number, count: number): string[] {
        return Array.from({ length: count }, (_, index) => {
            // number all but the first, trimming the stem so the handle fits
            const number = first + index;
            const suffix = number === 1 ? "" : `-${number}`;
            const trimmed = stem.slice(0, HANDLE_LENGTH - suffix.length).replace(/-+$/u, "");

            return `${trimmed}${suffix}`;
        });
    },
});
/** An account's handle. */
export type AccountHandle = schema.Infer<typeof AccountHandle>;

/** Whether a handle may be taken. */
export const HandleAvailability = schema.enum(["available", "taken", "invalid"]);
/** Whether a handle may be taken. */
export type HandleAvailability = schema.Infer<typeof HandleAvailability>;
