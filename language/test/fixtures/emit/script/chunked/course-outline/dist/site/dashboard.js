import { readCourseOutlineEntry } from "./course-outline.js";
const dashboardOutlineEntry = readCourseOutlineEntry(
    "course/03-rendering/02-static-layouts.ts",
);
export const dashboardOutlineState = {
    sectionNumber: dashboardOutlineEntry.sectionNumber,
    lessonNumber: dashboardOutlineEntry.lessonNumber,
    lessonKey: dashboardOutlineEntry.lessonKey,
    lessonSlug: dashboardOutlineEntry.lessonSlug,
};
//# sourceMappingURL=./dashboard.js.map
