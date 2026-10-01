import type { BucketCondition, BucketFile } from "../bucket/index.ts";
import { optional } from "./operation.ts";

/** The milliseconds of one second, the resolution of HTTP dates. */
const SECOND = 1000;

/** An HTTP entity tag, compared strongly unless weak comparison applies. */
export interface EntityTag {
    /** The unquoted tag. */
    etag: string;
    /** Whether the tag carries the W/ weakness prefix. */
    isWeak: boolean;
}

/** An HTTP entity tag, compared strongly unless weak comparison applies. */
export const EntityTag = { readList };

/** The conditional headers of an S3 request, with entity tags as HTTP lists. */
export interface S3Condition {
    /** The If-Match tags, or an asterisk matching any existing file. */
    etagMatches?: EntityTag[] | "*";
    /** The If-None-Match tags, or an asterisk requiring an absent file. */
    etagDoesNotMatch?: EntityTag[] | "*";
    /** The If-Unmodified-Since time. */
    uploadedBefore?: Date;
    /** The If-Modified-Since time. */
    uploadedAfter?: Date;
}

/** A bucket condition that keeps the request's conditions atomic, or the file they already fail against. */
export type S3Resolution = { onlyIf?: BucketCondition } | { failed: BucketFile | null };

/** The conditional headers of an S3 request, with entity tags as HTTP lists. */
export const S3Condition = { matches, resolve };

/** Read an HTTP entity-tag list, or an asterisk, accepting unquoted tags as S3 does. */
function readList(value: string): EntityTag[] | "*" {
    // match the asterisk alone
    if (value.trim() === "*") {
        return "*";
    }

    // read quoted tags, which may contain commas, and bare tokens
    return Array.from(value.matchAll(/(W\/)?"([^"]*)"|([^\s,"]+)/g), (match) => ({
        etag: match[2] ?? match[3]!,
        isWeak: match[1] !== undefined,
    }));
}

/** Evaluate the conditions against a file at HTTP's whole-second resolution and precedence. */
function matches(file: BucketFile | null, condition?: S3Condition): boolean {
    if (condition === undefined) {
        return true;
    }
    if (file === null) {
        return condition.etagMatches === undefined && condition.uploadedAfter === undefined;
    }

    // compare If-Match strongly and If-None-Match weakly, each overriding its date condition
    const isMatching =
        condition.etagMatches === undefined || includes(condition.etagMatches, file.etag, false);
    const isDifferent =
        condition.etagDoesNotMatch === undefined ||
        !includes(condition.etagDoesNotMatch, file.etag, true);
    const uploaded = seconds(file.uploaded);
    const isAfter =
        condition.uploadedAfter === undefined ||
        condition.etagDoesNotMatch !== undefined ||
        uploaded > seconds(condition.uploadedAfter);
    const isBefore =
        condition.uploadedBefore === undefined ||
        condition.etagMatches !== undefined ||
        uploaded < seconds(condition.uploadedBefore);

    return isMatching && isDifferent && isAfter && isBefore;
}

/** Translate conditions for the bucket, or pin a tag list the bucket cannot compare to the checked file. */
async function resolve(
    condition: S3Condition | undefined,
    head: () => Promise<BucketFile | null>,
): Promise<S3Resolution> {
    if (condition === undefined) {
        return {};
    }

    // translate an asterisk or a single tag the bucket compares the same way
    const { etagMatches, etagDoesNotMatch } = condition;
    if (isDirect(etagMatches, false) && isDirect(etagDoesNotMatch, true)) {
        return {
            onlyIf: {
                ...optional("etagMatches", etagMatches && directTag(etagMatches)),
                ...optional("etagDoesNotMatch", etagDoesNotMatch && directTag(etagDoesNotMatch)),
                ...optional("uploadedBefore", condition.uploadedBefore),
                ...optional("uploadedAfter", condition.uploadedAfter),
                secondsGranularity: true,
            },
        };
    }

    // evaluate other lists against the current file, then require that exact file
    const current = await head();
    if (!matches(current, condition)) {
        return { failed: current };
    }

    return {
        onlyIf: current === null ? { etagDoesNotMatch: "*" } : { etagMatches: current.etag },
    };
}

/** Whether a tag list is absent, an asterisk, or one tag the bucket compares as the request does. */
function isDirect(tags: EntityTag[] | "*" | undefined, isWeakComparison: boolean): boolean {
    return (
        tags === undefined ||
        tags === "*" ||
        (tags.length === 1 && (isWeakComparison || !tags[0]!.isWeak))
    );
}

/** The single unquoted tag or asterisk of a direct tag list. */
function directTag(tags: EntityTag[] | "*"): string {
    return tags === "*" ? "*" : tags[0]!.etag;
}

/** Whether a tag list includes a file's tag, where strong comparison never matches weak tags. */
function includes(tags: EntityTag[] | "*", etag: string, isWeakComparison: boolean): boolean {
    return (
        tags === "*" || tags.some((tag) => tag.etag === etag && (isWeakComparison || !tag.isWeak))
    );
}

/** Truncate a time to whole seconds. */
function seconds(time: Date): number {
    return Math.floor(time.getTime() / SECOND);
}
