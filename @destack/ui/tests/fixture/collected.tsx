import { carouselTripPhotos } from "../../src/carousel/carousel.example.tsx";
import { dialogDefault } from "../../src/dialog/dialog.example.tsx";
import { eventCalendarLaunch } from "../../src/event-calendar/event-calendar.example.tsx";
import { popoverImageSize } from "../../src/popover/popover.example.tsx";
import { resizablePanelGroupNotebookSplit } from "../../src/resizable/resizable.example.tsx";
import { sliderWorkingHours } from "../../src/slider/slider.example.tsx";
import { sortableLongList } from "../../src/sortable/sortable.example.tsx";
import { timelinePlan } from "../../src/timeline/timeline.example.tsx";
import { virtualListLog } from "../../src/virtual-list/virtual-list.example.tsx";

/** Render a carousel, which reads its slides. */
export function Carousel() {
    return carouselTripPhotos.render?.();
}

/** Render a dialog, which reads its description. */
export function Dialog() {
    return dialogDefault.render?.();
}

/** Render a calendar, which reads the views its view parts offer. */
export function EventCalendar() {
    return eventCalendarLaunch.render?.();
}

/** Render a popover, which reads its title and description. */
export function Popover() {
    return popoverImageSize.render?.();
}

/** Render a resizable group, which reads its panels and handles. */
export function Resizable() {
    return resizablePanelGroupNotebookSplit.render?.();
}

/** Render a range slider, which reads its thumbs. */
export function Slider() {
    return sliderWorkingHours.render?.();
}

/** Render a timeline, which reads its columns. */
export function Timeline() {
    return timelinePlan.render?.();
}

/** Render a sortable, which reads its items and its virtual list's window. */
export function Sortable() {
    return sortableLongList.render?.();
}

/** Render a virtual list, which measures lengths by elements. */
export function VirtualList() {
    return virtualListLog.render?.();
}
