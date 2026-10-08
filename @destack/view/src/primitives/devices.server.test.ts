import { expect, test } from "@destack/test";
import { createCameras, createDevices, createMicrophones, createSpeakers } from "./devices.ts";

test("list no devices on the server", () => {
    expect([
        createDevices()(),
        createMicrophones()(),
        createSpeakers()(),
        createCameras()(),
    ]).toEqual([[], [], [], []]);
});
