import { expect, test } from "@destack/test";
import {
    createAccelerometer,
    createBattery,
    createCompass,
    createGyroscope,
    createSensor,
    type GenericSensor,
    makeCompass,
} from "./sensors.ts";

/** A sensor the server never constructs. */
class UnusedSensor extends EventTarget implements GenericSensor {
    /** Whether the sensor has a reading. */
    readonly hasReading = false;
    /** When the last reading was taken. */
    readonly timestamp = undefined;

    /** Read whether the sensor runs, which it never does. */
    get activated(): boolean {
        return false;
    }

    /** Start reading. */
    start(): void {}

    /** Stop reading. */
    stop(): void {}
}

test("read no sensors on the server", () => {
    const gyroscope = createGyroscope();
    const compass = createCompass();

    expect([
        createAccelerometer()(),
        [gyroscope.alpha, gyroscope.beta, gyroscope.gamma],
        createSensor(UnusedSensor)(),
        makeCompass(() => {}),
        [compass.x, compass.y, compass.z],
        createBattery()(),
    ]).toEqual([undefined, [null, null, null], undefined, null, [null, null, null], undefined]);
});
