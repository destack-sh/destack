import { z } from "zod";
import { defineSchema } from "../inspect/schema.ts";

/** A calendar version: year, month and release, with an optional nightly build sequence. */
const VERSION = /^(\d{4})\.([1-9]|1[0-2])\.(0|[1-9]\d*)(?:-nightly\.(0|[1-9]\d*))?$/;

/** A package release version, such as 2026.9.0 or the nightly build 2026.10.0-nightly.3. */
export const Version = Object.assign(defineSchema(z.string().regex(VERSION)), {
    /** Order two versions by release, nightly builds before their release. */
    compare(left: string, right: string): number {
        // find the first differing component
        const first = Version.components(left);
        const second = Version.components(right);
        const index = first.findIndex((component, position) => component !== second[position]);

        return index === -1 ? 0 : Math.sign(first[index]! - second[index]!);
    },

    /** Read the year, month, release, stable flag and nightly sequence of a version. */
    components(version: string): readonly number[] {
        // read the calendar numbers and the nightly sequence
        const match = VERSION.exec(version);
        if (match === null) {
            throw new TypeError(`invalid version: ${version}`);
        }
        const nightly = match[4] === undefined ? undefined : Number(match[4]);

        return [...match.slice(1, 4).map(Number), nightly === undefined ? 1 : 0, nightly ?? 0];
    },
});

/** A package release version. */
export type Version = z.infer<typeof Version>;
