import { type Localization, t } from "@destack/locale";
import { type ObjectFileLimits, isAccepted } from "@destack/object";

import { FileSize } from "./size.ts";

/** A file that does not fit, and why: its media type, its size, or too many at once. */
export interface Refusal {
    /** The file. */
    readonly file: File;
    /** Why it was refused. */
    readonly reason: "type" | "size" | "count";
    /** The limits it did not fit. */
    readonly limits: ObjectFileLimits;
}

/** Report why a file does not fit limits, absent for one that does. */
export function refusalOf(file: File, limits: ObjectFileLimits): Refusal | undefined {
    if (!isAccepted(file.type, limits)) {
        return { file, reason: "type", limits };
    } else if (limits.size !== undefined && file.size > limits.size) {
        return { file, reason: "size", limits };
    }

    return undefined;
}

/** Write why a file was refused, beside its name. */
export function describeReason({ reason, limits }: Refusal, locale: Localization): string {
    // write the kind, the most bytes, or that one was asked for
    if (reason === "type") {
        return locale.render(t`Another kind of file`);
    } else if (reason === "size") {
        const most = FileSize.write(limits.size ?? 0, locale.number);

        return locale.render(t`Larger than ${most}`);
    }

    return locale.render(t`Choose one file`);
}

/** Write why a file was refused, with its name. */
export function describeRefusal({ file, reason, limits }: Refusal, locale: Localization): string {
    // write the file's kind, the most bytes, or that one was asked for
    if (reason === "type") {
        return locale.render(t`${file.name} is another kind of file`);
    } else if (reason === "size") {
        const most = FileSize.write(limits.size ?? 0, locale.number);

        return locale.render(t`${file.name} is larger than ${most}`);
    }

    return locale.render(t`Choose one file`);
}

/** Write what limits take, such as images or PDF, up to 20 MB, empty for anything. */
export function describeLimits(
    { accept = [], size }: ObjectFileLimits,
    locale: Localization,
): string {
    // write the types from a capital on, then the size
    const listed = locale.list(
        accept.map((type) => describeType(type, locale)),
        { type: "disjunction" },
    );
    const types = listed.charAt(0).toLocaleUpperCase(locale.tag) + listed.slice(1);
    if (size === undefined) {
        return types;
    }
    const most = FileSize.write(size, locale.number);

    return types === ""
        ? locale.render(t`Up to ${most}`)
        : locale.render(t`${types}, up to ${most}`);
}

/** Write a media type, or a range of them, as people call it. */
function describeType(type: string, locale: Localization): string {
    if (type === "image/*") {
        return locale.render(t`images`);
    } else if (type === "video/*") {
        return locale.render(t`videos`);
    } else if (type === "audio/*") {
        return locale.render(t`audio`);
    } else if (type === "text/*") {
        return locale.render(t`text`);
    }
    const [, subtype = type] = type.split("/");

    return (subtype.split("+")[0] ?? subtype).toUpperCase();
}
