import { isServer } from "@solidjs/web";
import { type Accessor, createMemo, createSignal, onCleanup } from "solid-js";

/** Follow the media devices, listed again as they change, each kind listed once without labels until it is permitted. */
export function createDevices(): Accessor<MediaDeviceInfo[]> {
    // list nothing on the server
    if (isServer) {
        return () => [];
    }

    // list the devices now and on each change, reporting a refused listing
    const [devices, setDevices] = createSignal<MediaDeviceInfo[]>([], { ownedWrite: true });
    const enumerate = (): void => {
        navigator.mediaDevices.enumerateDevices().then((listed) => setDevices(listed), reportError);
    };
    enumerate();
    navigator.mediaDevices.addEventListener("devicechange", enumerate);
    onCleanup(() => navigator.mediaDevices.removeEventListener("devicechange", enumerate));

    return devices;
}

/** Follow the microphones. */
export function createMicrophones(): Accessor<MediaDeviceInfo[]> {
    return createDevicesOfKind("audioinput");
}

/** Follow the speakers, whose identifiers `setSinkId` takes once microphones are permitted. */
export function createSpeakers(): Accessor<MediaDeviceInfo[]> {
    return createDevicesOfKind("audiooutput");
}

/** Follow the cameras. */
export function createCameras(): Accessor<MediaDeviceInfo[]> {
    return createDevicesOfKind("videoinput");
}

/** Follow the devices of one kind, changing only when the list does, through a memo on every side to keep hydration keys aligned. */
function createDevicesOfKind(kind: MediaDeviceKind): Accessor<MediaDeviceInfo[]> {
    const devices = createDevices();

    return createMemo(() => devices().filter((device) => device.kind === kind), {
        equals: (previous, next) =>
            previous.length === next.length && previous.every((device) => next.includes(device)),
    });
}
