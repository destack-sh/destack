import { expect, test } from "@destack/test";
import { createPulse, createVibrate, isVibrationSupported } from "./vibrate.ts";

test("vibrate nothing on the server", () => {
    expect([
        isVibrationSupported(),
        createVibrate(() => 200).supported,
        createPulse(2).pulsing(),
    ]).toEqual([false, false, false]);
});
