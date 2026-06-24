import type { StringId } from "../../../_generated/core/string.js";
import type { Path } from "../../../_generated/dir/tree/path.js";

export const PathImpl = {
    /** Return the final path segment. */
    lastSegment(path: Path): StringId | undefined {
        return path.segments.at(-1);
    },
};
