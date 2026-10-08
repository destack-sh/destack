import { createActiveElement } from "./active-element.ts";
import { createAudio } from "./audio.ts";
import { createElementBounds } from "./bounds.ts";
import { createClipboard } from "./clipboard.ts";
import { createConnectivitySignal, createNetworkInformation } from "./connectivity.ts";
import { createBodyCursor, createDragCursor, createElementCursor } from "./cursor.ts";
import { createCameras, createDevices } from "./devices.ts";
import { createEventDispatcher } from "./event-dispatcher.ts";
import {
    createEventListener,
    createEventListenerMap,
    createEventSignal,
} from "./event-listener.ts";
import {
    createFavicon,
    createFaviconBadge,
    createFaviconProgress,
    createFaviconScheme,
} from "./favicon.ts";
import { createAutofocus, createFocusSignal } from "./focus.ts";
import { createFullscreen } from "./fullscreen.ts";
import { createGeolocationWatcher } from "./geolocation.ts";
import { createIdleTimer } from "./idle.ts";
import {
    createIntersectionObserver,
    createViewportObserver,
    createVisibilityObserver,
} from "./intersection-observer.ts";
import { createKeyDown, createKeyHold, createShortcut, useKeyDownList } from "./keyboard.ts";
import { createBreakpoints, createMediaQuery, createPrefersDark, usePrefersDark } from "./media.ts";
import { createMutationObserver } from "./mutation-observer.ts";
import { createNotificationPermission } from "./notification.ts";
import { createOrientation } from "./orientation.ts";
import { createPageVisibility } from "./page-visibility.ts";
import { createPermission } from "./permission.ts";
import { createPointerList, createPointerListeners, createPointerPosition } from "./pointer.ts";
import { createMs, createRAF } from "./raf.ts";
import { createElementSize, createResizeObserver, createWindowSize } from "./resize-observer.ts";
import { createScheduled, debounce } from "./scheduled.ts";
import { createPreventScroll, createScrollPosition, useWindowScrollPosition } from "./scroll.ts";
import { createSelection } from "./selection.ts";
import { createBattery } from "./sensors.ts";
import { createWebShare } from "./share.ts";
import { createRemSize } from "./styles.ts";
import { createIntervalCounter, createPolled, createTimer } from "./timer.ts";
import { createDropzone, createFilePicker } from "./upload.ts";
import { createVibrate } from "./vibrate.ts";
import { createVideo, createVideoFrameCallback } from "./video.ts";
import { createPageLeaveBlocker } from "../router/leave.ts";

/** The element a fixture's ref receives once rendered, none on the server. */
let element: HTMLElement | undefined;

/** Read the element the fixture's ref received. */
const target = () => element;

/** Read the elements the fixture's ref received, none before rendering. */
const targets = () => (element === undefined ? [] : [element]);

/** Render a paragraph after a primitive ran, whose hydration key shows whether the primitive kept the keys aligned. */
function Probe(properties: { readonly name: string }) {
    return (
        <p
            ref={(node) => {
                element = node;
            }}
        >
            {properties.name}
        </p>
    );
}

/** Run the active element primitives before a probe. */
export function ActiveElement() {
    // run the primitives
    createActiveElement();

    return <Probe name="active element" />;
}

/** Run the audio primitives before a probe. */
export function Audio() {
    // run the primitives
    createAudio(() => undefined);

    return <Probe name="audio" />;
}

/** Run the element bounds primitives before a probe. */
export function ElementBounds() {
    // run the primitives
    createElementBounds(target);

    return <Probe name="element bounds" />;
}

/** Run the clipboard primitives before a probe. */
export function Clipboard() {
    // run the primitives
    createClipboard(undefined, true);

    return <Probe name="clipboard" />;
}

/** Run the connectivity primitives before a probe. */
export function Connectivity() {
    // run the primitives
    createConnectivitySignal();
    createNetworkInformation();

    return <Probe name="connectivity" />;
}

/** Run the cursor primitives before a probe. */
export function Cursor() {
    // run the primitives
    createElementCursor(target, "pointer");
    createBodyCursor(() => false);
    createDragCursor(target);

    return <Probe name="cursor" />;
}

/** Run the devices primitives before a probe. */
export function Devices() {
    // run the primitives
    createDevices();
    createCameras();

    return <Probe name="devices" />;
}

/** Run the event dispatcher primitives before a probe. */
export function EventDispatcher() {
    // run the primitives
    createEventDispatcher({});

    return <Probe name="event dispatcher" />;
}

/** Run the event listener primitives before a probe. */
export function EventListener() {
    // run the primitives
    createEventListener(target, "click", () => {});
    createEventListenerMap(targets, { click: () => {} });
    createEventSignal(targets, "click");

    return <Probe name="event listener" />;
}

/** Run the favicon primitives before a probe. */
export function Favicon() {
    // run the primitives
    createFavicon("/icon.svg");
    createFaviconBadge("/icon.svg", 1);
    createFaviconProgress("/icon.svg", 0.5);
    createFaviconScheme({ light: "/light.svg", dark: "/dark.svg" });

    return <Probe name="favicon" />;
}

/** Run the focus primitives before a probe. */
export function Focus() {
    // run the primitives
    createFocusSignal(() => document.body);
    createAutofocus(target);

    return <Probe name="focus" />;
}

/** Run the fullscreen primitives before a probe. */
export function Fullscreen() {
    // run the primitives
    createFullscreen(target);

    return <Probe name="fullscreen" />;
}

/** Run the geolocation primitives before a probe. */
export function Geolocation() {
    // run the primitives
    createGeolocationWatcher(false);

    return <Probe name="geolocation" />;
}

/** Run the idle timer primitives before a probe. */
export function IdleTimer() {
    // run the primitives
    createIdleTimer({ startManually: true });

    return <Probe name="idle timer" />;
}

/** Run the intersection observer primitives before a probe. */
export function IntersectionObserver() {
    // run the primitives
    createIntersectionObserver(() => []);
    createViewportObserver(
        () => [],
        () => {},
    );
    createVisibilityObserver(target);

    return <Probe name="intersection observer" />;
}

/** Run the keyboard primitives before a probe. */
export function Keyboard() {
    // run the primitives
    createKeyHold("Shift");
    createKeyDown("Enter", () => {});
    createShortcut(["Control", "K"], () => {});
    useKeyDownList();

    return <Probe name="keyboard" />;
}

/** Run the media primitives before a probe. */
export function Media() {
    // run the primitives
    createMediaQuery("(min-width: 40rem)");
    createPrefersDark();
    usePrefersDark();
    createBreakpoints({ small: "40rem", large: "80rem" });

    return <Probe name="media" />;
}

/** Run the mutation observer primitives before a probe. */
export function MutationObserver() {
    // run the primitives
    createMutationObserver(
        () => [],
        { childList: true },
        () => {},
    );

    return <Probe name="mutation observer" />;
}

/** Run the notification primitives before a probe. */
export function Notification() {
    // run the primitives
    createNotificationPermission();

    return <Probe name="notification" />;
}

/** Run the orientation primitives before a probe. */
export function Orientation() {
    // run the primitives
    createOrientation();

    return <Probe name="orientation" />;
}

/** Run the page visibility primitives before a probe. */
export function PageVisibility() {
    // run the primitives
    createPageVisibility();

    return <Probe name="page visibility" />;
}

/** Run the permission primitives before a probe. */
export function Permission() {
    // run the primitives
    createPermission("geolocation");

    return <Probe name="permission" />;
}

/** Run the pointer primitives before a probe. */
export function Pointer() {
    // run the primitives
    createPointerListeners({ onDown: () => {} });
    createPointerPosition();
    createPointerList();

    return <Probe name="pointer" />;
}

/** Run the frames primitives before a probe. */
export function Frames() {
    // run the primitives
    createRAF(() => {});
    createMs(60);

    return <Probe name="frames" />;
}

/** Run the resize observer primitives before a probe. */
export function ResizeObserver() {
    // run the primitives
    createResizeObserver(target, () => {});
    createWindowSize();
    createElementSize(target);

    return <Probe name="resize observer" />;
}

/** Run the scheduled primitives before a probe. */
export function Scheduled() {
    // run the primitives
    createScheduled((callback) => debounce(callback, 10));

    return <Probe name="scheduled" />;
}

/** Run the scroll primitives before a probe. */
export function Scroll() {
    // run the primitives
    createScrollPosition(target);
    useWindowScrollPosition();
    createPreventScroll({ element: target, enabled: () => false });

    return <Probe name="scroll" />;
}

/** Run the selection primitives before a probe. */
export function Selection() {
    // run the primitives
    createSelection();

    return <Probe name="selection" />;
}

/** Run the battery primitives before a probe. */
export function Battery() {
    // run the primitives
    createBattery();

    return <Probe name="battery" />;
}

/** Run the share primitives before a probe. */
export function Share() {
    // run the primitives
    createWebShare();

    return <Probe name="share" />;
}

/** Run the rem size primitives before a probe. */
export function RemSize() {
    // run the primitives
    createRemSize();

    return <Probe name="rem size" />;
}

/** Run the timer primitives before a probe. */
export function Timer() {
    // run the primitives
    createTimer(
        () => {},
        () => false,
        setTimeout,
    );
    createPolled(
        () => 1,
        () => false,
    );
    createIntervalCounter(() => false);

    return <Probe name="timer" />;
}

/** Run the upload primitives before a probe. */
export function Upload() {
    // run the primitives
    createFilePicker();
    createDropzone();

    return <Probe name="upload" />;
}

/** Run the vibrate primitives before a probe. */
export function Vibrate() {
    // run the primitives
    createVibrate(100);

    return <Probe name="vibrate" />;
}

/** Run the video primitives before a probe. */
export function Video() {
    // run the primitives
    createVideo(() => undefined);
    createVideoFrameCallback(
        () => undefined,
        () => {},
    );

    return <Probe name="video" />;
}

/** Run the page leave primitives before a probe. */
export function PageLeave() {
    // run the primitives
    createPageLeaveBlocker(() => false);

    return <Probe name="page leave" />;
}
