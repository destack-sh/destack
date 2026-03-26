export const missingOutlineNumber = "missing";
export function splitOutlinePath(filePath) {
    return filePath.split("/");
}
export function getOutlineSegmentFromEnd(filePath, offsetFromEnd) {
    const segments = splitOutlinePath(filePath);
    return segments[segments.length - offsetFromEnd];
}
export function getOutlineNumberPrefix(pathSegment) {
    return pathSegment.split("-")[0];
}
export function getOutlineNumber(pathSegment) {
    const numberSegment = getOutlineNumberPrefix(pathSegment);
    if (numberSegment === pathSegment) {
        return missingOutlineNumber;
    }
    return numberSegment;
}
export function readCourseOutlineEntry(filePath) {
    const sectionSegment = getOutlineSegmentFromEnd(filePath, 2);
    const lessonSegment = getOutlineSegmentFromEnd(filePath, 1);
    if (!sectionSegment || !lessonSegment) {
        return {
            sectionNumber: missingOutlineNumber,
            lessonNumber: missingOutlineNumber,
            lessonKey: `${missingOutlineNumber}-${missingOutlineNumber}`,
            lessonSlug: missingOutlineNumber,
        };
    }
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
//# sourceMappingURL=./course-outline.js.map
