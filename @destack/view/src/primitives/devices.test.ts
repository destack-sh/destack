import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createCameras, createDevices, createMicrophones, createSpeakers } from "./devices.ts";

/** Make a device of a kind. */
function device(kind: MediaDeviceKind, label: string): MediaDeviceInfo {
    const values = { deviceId: label, groupId: "group", kind, label };

    return { ...values, toJSON: () => values };
}

/** A stand-in for the browser's media devices, whose list the test changes. */
class StubDevices extends EventTarget {
    /** The devices plugged in. */
    devices: MediaDeviceInfo[] = [
        device("audioinput", "microphone"),
        device("videoinput", "camera"),
    ];

    /** List the devices. */
    async enumerateDevices(): Promise<MediaDeviceInfo[]> {
        return this.devices;
    }

    /** Plug a device in, telling the listeners. */
    plug(added: MediaDeviceInfo): void {
        this.devices = [...this.devices, added];
        this.dispatchEvent(new Event("devicechange"));
    }
}

/** The media devices of the current test. */
let media = new StubDevices();

beforeEach(() => {
    media = new StubDevices();
    Object.defineProperty(navigator, "mediaDevices", { configurable: true, get: () => media });
});

afterEach(() => {
    Reflect.deleteProperty(navigator, "mediaDevices");
});

/** Wait for the listing to answer. */
async function settle(): Promise<void> {
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
    flush();
}

test("follow the devices, and each kind of them, as devices are plugged in", async () => {
    const { lists, dispose } = createRoot((disposeRoot) => ({
        lists: [createDevices(), createMicrophones(), createSpeakers(), createCameras()],
        dispose: disposeRoot,
    }));
    const labels = (): string[][] => lists.map((list) => list().map((item) => item.label));
    const before = labels();
    await settle();
    const listed = labels();
    media.plug(device("audiooutput", "speaker"));
    await settle();
    const plugged = labels();
    dispose();

    expect([before, listed, plugged]).toEqual([
        [[], [], [], []],
        [["microphone", "camera"], ["microphone"], [], ["camera"]],
        [["microphone", "camera", "speaker"], ["microphone"], ["speaker"], ["camera"]],
    ]);
});
