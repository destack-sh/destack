import type { CommentNewlines } from "../../../_generated/dir/source/comment.js";

/** Bit flag for a leading newline. */
const LEADING = 1 << 0;
/** Bit flag for a trailing newline. */
const TRAILING = 1 << 1;

export const CommentNewlinesImpl = {
    /** Return whether a comment has a leading newline. */
    hasLeadingNewline(newlines: CommentNewlines): boolean {
        return (newlines.bits & LEADING) !== 0;
    },

    /** Return whether a comment has a trailing newline. */
    hasTrailingNewline(newlines: CommentNewlines): boolean {
        return (newlines.bits & TRAILING) !== 0;
    },
};
