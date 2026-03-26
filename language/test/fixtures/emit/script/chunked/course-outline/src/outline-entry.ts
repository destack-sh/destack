import {
    getOutlineNumberPrefix,
    getOutlineSegmentFromEnd,
} from "./path-segments.ts";

/** The fallback outline number for incomplete outline paths. */
export const missingOutlineNumber = "missing";

/** Read the numeric outline prefix from a section or lesson segment. */
export function getOutlineNumber(pathSegment: string) {
    const numberSegment = getOutlineNumberPrefix(pathSegment);

    // fall back when the path segment has no prefix
    if (numberSegment === pathSegment) {
        return missingOutlineNumber;
    }

    return numberSegment;
}

/** Read the derived fields for one course outline file path. */
export function readCourseOutlineEntry(filePath: string) {
    // locate the outline path segments
    const sectionSegment = getOutlineSegmentFromEnd(filePath, 2);
    const lessonSegment = getOutlineSegmentFromEnd(filePath, 1);

    // fall back for incomplete paths
    if (!sectionSegment || !lessonSegment) {
        return {
            sectionNumber: missingOutlineNumber,
            lessonNumber: missingOutlineNumber,
            lessonKey: `${missingOutlineNumber}-${missingOutlineNumber}`,
            lessonSlug: missingOutlineNumber,
        };
    }

    // build the derived outline fields
    const sectionNumber = getOutlineNumber(sectionSegment);
    const lessonNumber = getOutlineNumber(lessonSegment);
    const lessonSegments = lessonSegment.split("-");
    const lessonSlug = lessonSegments.slice(1).join("-");

    return {
        sectionNumber,
        lessonNumber,
        lessonKey: `${sectionNumber}-${lessonNumber}`,
        lessonSlug,
    };
}
