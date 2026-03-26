/** Split an outline file path into its path segments. */
export function splitOutlinePath(filePath: string): string[] {
    return filePath.split("/");
}

/** Read a path segment from the end of an outline file path. */
export function getOutlineSegmentFromEnd(
    filePath: string,
    offsetFromEnd: number,
): string | undefined {
    const segments = splitOutlinePath(filePath);

    return segments[segments.length - offsetFromEnd];
}

/** Read the numeric outline prefix from a path segment. */
export function getOutlineNumberPrefix(pathSegment: string): string {
    return pathSegment.split("-")[0];
}
