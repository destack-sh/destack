import type { BucketFile } from "./file.ts";

/** Preconditions evaluated atomically with a read or write. */
export interface BucketCondition {
    /** Match one unquoted entity tag, or any existing file with an asterisk. */
    etagMatches?: string;
    /** Exclude one unquoted entity tag, or require an absent file with an asterisk. */
    etagDoesNotMatch?: string;
    /** Require an upload earlier than this time. */
    uploadedBefore?: Date;
    /** Require an upload later than this time. */
    uploadedAfter?: Date;
    /** Compare upload times at HTTP's whole-second resolution. */
    secondsGranularity?: boolean;
}

/** Preconditions evaluated atomically with a read or write. */
export const BucketCondition = { matches };

/** Evaluate file preconditions with HTTP entity-tag precedence. */
function matches(file: BucketFile | null, condition?: BucketCondition): boolean {
    if (!condition) {
        return true;
    }
    if (!file) {
        return condition.etagMatches === undefined && condition.uploadedAfter === undefined;
    }

    // evaluate entity tags, which override their associated date conditions
    const isMatching =
        condition.etagMatches === undefined || matchesEtag(condition.etagMatches, file.etag);
    const isDifferent =
        condition.etagDoesNotMatch === undefined ||
        !matchesEtag(condition.etagDoesNotMatch, file.etag);
    const divisor = condition.secondsGranularity === true ? 1000 : 1;
    const uploaded = Math.floor(file.uploaded.getTime() / divisor);
    const isAfter =
        condition.uploadedAfter === undefined ||
        condition.etagDoesNotMatch !== undefined ||
        uploaded > Math.floor(condition.uploadedAfter.getTime() / divisor);
    const isBefore =
        condition.uploadedBefore === undefined ||
        condition.etagMatches !== undefined ||
        uploaded < Math.floor(condition.uploadedBefore.getTime() / divisor);

    return isMatching && isDifferent && isAfter && isBefore;
}

/** Compare one unquoted entity tag, or an asterisk matching any file, with a file's tag. */
function matchesEtag(expected: string, etag: string): boolean {
    return expected === "*" || expected === etag;
}
