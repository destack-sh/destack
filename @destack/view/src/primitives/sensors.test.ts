import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, flush } from "solid-js";
import {
    type AccelerometerReading,
    type BatteryReading,
    createAccelerometer,
    createBattery,
    createCompass,
    createGyroscope,
    createSensor,
    type GyroscopeReading,
    makeAccelerometer,
    makeBattery,
    makeCompass,
    makeGyroscope,
    makeSensor,
} from "./sensors.ts";

/** A stand-in Generic Sensor API sensor the test reads. */
class StubSensor extends EventTarget {
    /** Whether the sensor runs, behind the interface's name. */
    isActive = false;
    /** Whether the sensor has a reading. */
    hasReading = false;
    /** When the last reading was taken. */
    timestamp: number | undefined = undefined;
    /** The reading along the x axis. */
    x: number | null = null;
    /** The reading along the y axis. */
    y: number | null = null;
    /** The reading along the z axis. */
    z: number | null = null;
    /** The options the sensor was made with. */
    readonly options: unknown;

    /** Keep the options and remember the sensor. */
    constructor(options?: unknown) {
        super();
        this.options = options;
        sensors.push(this);
    }

    /** Read whether the sensor runs. */
    get activated(): boolean {
        return this.isActive;
    }

    /** Start reading. */
    start(): void {
        this.isActive = true;
    }

    /** Stop reading. */
    stop(): void {
        this.isActive = false;
    }

    /** Take a reading. */
    read(x: number | null, y: number | null, z: number | null): void {
        Object.assign(this, { x, y, z, hasReading: true });
        this.dispatchEvent(new Event("reading"));
    }
}

/** A stand-in sensor whose construction the browser refuses. */
class RefusedSensor extends StubSensor {
    /** Refuse to construct. */
    constructor() {
        super();
        throw new DOMException("sensor not allowed", "NotAllowedError");
    }
}

/** The sensors made in the current test. */
let sensors: StubSensor[] = [];

beforeEach(() => {
    vi.useFakeTimers();
    sensors = [];
});

afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    Reflect.deleteProperty(navigator, "getBattery");
});

/** Dispatch a device motion event. */
function move(x: number): void {
    const event = new Event("devicemotion");
    Object.assign(event, {
        acceleration: { x, y: 0, z: 0 },
        accelerationIncludingGravity: { x, y: 0, z: 9.8 },
    });
    window.dispatchEvent(event);
}

/** Dispatch a device orientation event. */
function orient(alpha: number | null, beta: number | null, gamma: number | null): void {
    const event = new Event("deviceorientation");
    Object.assign(event, { alpha, beta, gamma });
    window.dispatchEvent(event);
}

test("call back with acceleration at most once an interval, with or without gravity, until cleared", () => {
    const seen: AccelerometerReading[] = [];
    const clear = makeAccelerometer((reading) => seen.push(reading), { interval: 100 });
    const clearGravity = makeAccelerometer((reading) => seen.push(reading), {
        includeGravity: true,
        interval: 100,
    });
    move(1);
    move(2);
    vi.advanceTimersByTime(100);
    move(3);
    clear();
    clearGravity();
    move(4);

    expect(seen).toEqual([
        { x: 1, y: 0, z: 0 },
        { x: 1, y: 0, z: 9.8 },
        { x: 3, y: 0, z: 0 },
        { x: 3, y: 0, z: 9.8 },
    ]);
});

test("follow acceleration from undefined", () => {
    const observed = createRoot((disposeRoot) => {
        const acceleration = createAccelerometer();
        const before = acceleration();
        move(5);
        flush();
        const after = acceleration();
        disposeRoot();

        return [before, after];
    });

    expect(observed).toEqual([undefined, { x: 5, y: 0, z: 0 }]);
});

test("call back with orientation angles, keeping unreported ones null", () => {
    const seen: GyroscopeReading[] = [];
    const clear = makeGyroscope((reading) => seen.push(reading));
    orient(10, 20, 30);
    vi.advanceTimersByTime(100);
    orient(null, null, null);
    clear();

    expect(seen).toEqual([
        { alpha: 10, beta: 20, gamma: 30 },
        { alpha: null, beta: null, gamma: null },
    ]);
});

test("follow orientation angles from null", () => {
    const observed = createRoot((disposeRoot) => {
        const gyroscope = createGyroscope();
        const before = { ...gyroscope };
        orient(45, 90, 180);
        flush();
        const after = { alpha: gyroscope.alpha, beta: gyroscope.beta, gamma: gyroscope.gamma };
        disposeRoot();

        return [before, after];
    });

    expect(observed).toEqual([
        { alpha: null, beta: null, gamma: null },
        { alpha: 45, beta: 90, gamma: 180 },
    ]);
});

test("start a sensor, call back on each reading, and stop it when cleared", () => {
    const readings: (number | null)[] = [];
    const clear = makeSensor(StubSensor, (sensor) => readings.push(sensor.x), { frequency: 30 });
    const sensor = sensors[0];
    const started = [sensor?.activated, sensor?.options];
    sensor?.read(1, 2, 3);
    sensor?.read(4, 5, 6);
    clear();
    sensor?.read(7, 8, 9);

    expect([started, readings, sensor?.activated]).toEqual([
        [true, { frequency: 30 }],
        [1, 4],
        false,
    ]);
});

test("throw when the browser refuses a sensor", () => {
    expect(() => makeSensor(RefusedSensor, () => {})).toThrow(
        new DOMException("sensor not allowed", "NotAllowedError"),
    );
});

test("follow a sensor through every reading, though it stays the same object", () => {
    const observed = createRoot((disposeRoot) => {
        const sensor = createSensor(StubSensor);
        const before = sensor();
        sensors[0]?.read(1, 0, 0);
        flush();
        const first = sensor()?.x;
        sensors[0]?.read(2, 0, 0);
        flush();
        const second = sensor()?.x;
        disposeRoot();

        return [before, first, second, sensors[0]?.activated];
    });

    expect(observed).toEqual([undefined, 1, 2, false]);
});

test("follow the magnetometer, or nothing where the device has none", () => {
    // read without a magnetometer
    const missing = makeCompass(() => {});

    // read a magnetometer with options, keeping missing axes null
    vi.stubGlobal("Magnetometer", StubSensor);
    const observed = createRoot((disposeRoot) => {
        const compass = createCompass({ frequency: 10, referenceFrame: "screen" });
        const before = { x: compass.x, y: compass.y, z: compass.z };
        sensors[0]?.read(30, null, -12);
        flush();
        const after = { x: compass.x, y: compass.y, z: compass.z };
        disposeRoot();

        return { before, after, options: sensors[0]?.options, isActive: sensors[0]?.activated };
    });

    expect([missing, observed]).toEqual([
        null,
        {
            before: { x: null, y: null, z: null },
            after: { x: 30, y: null, z: -12 },
            options: { frequency: 10, referenceFrame: "screen" },
            isActive: false,
        },
    ]);
});

test("follow the battery once it answers and on each change, attaching nothing once stopped", async () => {
    // answer with a battery the test changes
    const battery = Object.assign(new EventTarget(), {
        charging: true,
        chargingTime: 60,
        dischargingTime: Number.POSITIVE_INFINITY,
        level: 0.5,
    });
    Object.defineProperty(navigator, "getBattery", {
        configurable: true,
        value: async () => battery,
    });
    const seen: BatteryReading[] = [];
    const clear = makeBattery((reading) => seen.push(reading));
    const clearEarly = makeBattery((reading) => seen.push(reading));
    clearEarly();
    await Promise.resolve();
    await Promise.resolve();
    battery.level = 0.6;
    battery.dispatchEvent(new Event("levelchange"));
    clear();
    battery.dispatchEvent(new Event("levelchange"));

    // follow it in a root
    const observed = createRoot((disposeRoot) => ({
        reading: createBattery(),
        dispose: disposeRoot,
    }));
    const before = observed.reading();
    await Promise.resolve();
    await Promise.resolve();
    flush();
    const after = observed.reading()?.level;
    observed.dispose();

    expect({ levels: seen.map((reading) => reading.level), before, after }).toEqual({
        levels: [0.5, 0.6],
        before: undefined,
        after: 0.6,
    });
});
