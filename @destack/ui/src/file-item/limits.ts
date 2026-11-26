import { type Localization, plural, t } from "@destack/locale";
import { type ObjectFileLimits, isAccepted } from "@destack/object";

import { FileSize } from "./size.ts";

/** What a place takes: the media types and sizes of a file field, and how many files. */
export interface FileLimits extends ObjectFileLimits {
    /** The fewest bytes of a file. */
    readonly minimum?: number;
    /** The most files, any number when absent. */
    readonly count?: number;
}

/** Why a file does not fit: its type, a folder, its size, too many, or the app's own reason. */
export type RefusalReason = "type" | "folder" | "size" | "minimum" | "count" | "custom";

/** A file that does not fit, and why. */
export interface Refusal {
    /** The file. */
    readonly file: File;
    /** Why it was refused. */
    readonly reason: RefusalReason;
    /** The limits it did not fit. */
    readonly limits: FileLimits;
    /** Why in the app's own words, for its own reason. */
    readonly message?: string;
}

/** Report why a file does not fit limits, absent for one that does. */
export function refusalOf(file: File, limits: FileLimits): Refusal | undefined {
    if (!isAccepted(file.type, limits)) {
        return { file, reason: "type", limits };
    } else if (limits.size !== undefined && file.size > limits.size) {
        return { file, reason: "size", limits };
    } else if (limits.minimum !== undefined && file.size < limits.minimum) {
        return { file, reason: "minimum", limits };
    }

    return undefined;
}

/** Write why a file was refused, beside its name. */
export function describeReason(refusal: Refusal, locale: Localization): string {
    // write the app's own words, the kind, the most or fewest bytes, or how many it takes
    const { reason, limits, message } = refusal;
    if (message !== undefined) {
        return message;
    } else if (reason === "type") {
        return locale.render(t`Another kind of file`);
    } else if (reason === "folder") {
        return locale.render(t.context("option")`A folder`);
    } else if (reason === "size") {
        const most = FileSize.write(limits.size ?? 0, locale.number);

        return locale.render(t`Larger than ${most}`);
    } else if (reason === "minimum") {
        const least = FileSize.write(limits.minimum ?? 0, locale.number);

        return locale.render(t`Smaller than ${least}`);
    }

    return describeCount(limits.count ?? 1, locale);
}

/** Write why a file was refused, with its name. */
export function describeRefusal(refusal: Refusal, locale: Localization): string {
    // write the app's own words, the kind, the most or fewest bytes, or how many it takes
    const { file, reason, limits, message } = refusal;
    if (message !== undefined) {
        return message;
    } else if (reason === "type") {
        return locale.render(t`${file.name} is another kind of file`);
    } else if (reason === "folder") {
        return locale.render(t`${file.name} is a folder`);
    } else if (reason === "size") {
        const most = FileSize.write(limits.size ?? 0, locale.number);

        return locale.render(t`${file.name} is larger than ${most}`);
    } else if (reason === "minimum") {
        const least = FileSize.write(limits.minimum ?? 0, locale.number);

        return locale.render(t`${file.name} is smaller than ${least}`);
    }

    return describeCount(limits.count ?? 1, locale);
}

/** Write how many files a place takes. */
function describeCount(count: number, locale: Localization): string {
    return locale.render(
        t`${plural(count, { one: "Choose one file", other: "Choose up to # files" })}`,
    );
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
        return locale.render(t.context("option")`images`);
    } else if (type === "video/*") {
        return locale.render(t.context("option")`videos`);
    } else if (type === "audio/*") {
        return locale.render(t.context("option")`audio`);
    } else if (type === "text/*") {
        return locale.render(t.context("option")`text`);
    }
    const [, subtype = type] = type.split("/");

    return (subtype.split("+")[0] ?? subtype).toUpperCase();
}
