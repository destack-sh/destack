import { isServer } from "@solidjs/web";
import { type Accessor, createSignal, onCleanup } from "solid-js";

/** How often a sensor reads, in readings a second. */
export type SensorOptions = { readonly frequency?: number };

/** A sensor of the Generic Sensor API, which the DOM types leave out. */
export interface GenericSensor extends EventTarget {
    /** Whether the sensor runs. */
    readonly activated: boolean;
    /** Whether the sensor has a reading. */
    readonly hasReading: boolean;
    /** When the last reading was taken. */
    readonly timestamp: DOMHighResTimeStamp | undefined;
    /** Start reading. */
    start(): void;
    /** Stop reading. */
    stop(): void;
}

/** A Generic Sensor API constructor, such as `Magnetometer`. */
export type SensorConstructor<Sensor extends GenericSensor, Options = SensorOptions> = new (
    options?: Options,
) => Sensor;

/** A motion reading, null where the device does not report acceleration. */
export type AccelerometerReading = DeviceMotionEventAcceleration | null;

/** An orientation reading in degrees, each angle null where the device does not report it. */
export type GyroscopeReading = {
    readonly alpha: number | null;
    readonly beta: number | null;
    readonly gamma: number | null;
};

/** How a compass reads. */
export type CompassOptions = SensorOptions & { readonly referenceFrame?: "device" | "screen" };

/** A magnetometer reading in microteslas, each axis null before the first reading. */
export type CompassReading = {
    readonly x: number | null;
    readonly y: number | null;
    readonly z: number | null;
};

/** The battery's state. */
export type BatteryReading = {
    /** Whether the battery charges. */
    readonly charging: boolean;
    /** The seconds until it is charged. */
    readonly chargingTime: number;
    /** The seconds until it is empty. */
    readonly dischargingTime: number;
    /** The charge, from zero to one. */
    readonly level: number;
};

/** A magnetometer of the Generic Sensor API. */
interface MagnetometerSensor extends GenericSensor {
    /** The field along the x axis. */
    readonly x: number | null;
    /** The field along the y axis. */
    readonly y: number | null;
    /** The field along the z axis. */
    readonly z: number | null;
}

/** The battery manager of the Battery Status API, which the DOM types leave out. */
interface BatteryManager extends EventTarget, BatteryReading {}

/** The default milliseconds between motion and orientation readings, ten a second. */
const READING_INTERVAL = 100;

/** The battery events that change its state. */
const BATTERY_EVENTS = [
    "chargingchange",
    "chargingtimechange",
    "dischargingtimechange",
    "levelchange",
] as const;

/** Start a Generic Sensor API sensor, calling back with each reading and reporting its errors, until the returned function is called. */
export function makeSensor<Sensor extends GenericSensor, Options = SensorOptions>(
    SensorClass: SensorConstructor<Sensor, Options>,
    onChange: (sensor: Sensor) => void,
    options?: Options,
): () => void {
    // construct and start the sensor, letting a refusal throw
    const sensor = new SensorClass(options);
    const read = (): void => onChange(sensor);
    sensor.addEventListener("reading", read);
    sensor.addEventListener("error", fail);
    sensor.start();

    return () => {
        sensor.removeEventListener("reading", read);
        sensor.removeEventListener("error", fail);
        sensor.stop();
    };
}

/** Follow a Generic Sensor API sensor through every reading, undefined until the first, until cleanup. */
export function createSensor<Sensor extends GenericSensor, Options = SensorOptions>(
    SensorClass: SensorConstructor<Sensor, Options>,
    options?: Options,
): Accessor<Sensor | undefined> {
    // read nothing on the server
    if (isServer) {
        return () => undefined;
    }
    const [sensor, setSensor] = createSignal<{ readonly current: Sensor } | undefined>(undefined, {
        ownedWrite: true,
        equals: false,
    });
    onCleanup(makeSensor(SensorClass, (current) => setSensor({ current }), options));

    return () => sensor()?.current;
}

/** Call back with the device's acceleration on motion, at most once an interval, until the returned function is called. */
export function makeAccelerometer(
    onChange: (acceleration: AccelerometerReading) => void,
    options: { readonly includeGravity?: boolean; readonly interval?: number } = {},
): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }
    const { includeGravity = false, interval = READING_INTERVAL } = options;

    return makeThrottledListener("devicemotion", interval, (event: DeviceMotionEvent) =>
        onChange(includeGravity ? event.accelerationIncludingGravity : event.acceleration),
    );
}

/** Follow the device's acceleration, undefined until the first motion, until cleanup. */
export function createAccelerometer(
    includeGravity = false,
    interval = READING_INTERVAL,
): Accessor<AccelerometerReading | undefined> {
    // read nothing on the server
    if (isServer) {
        return () => undefined;
    }
    const [acceleration, setAcceleration] = createSignal<AccelerometerReading | undefined>(
        undefined,
        { ownedWrite: true },
    );
    onCleanup(
        makeAccelerometer((reading) => setAcceleration(reading), { includeGravity, interval }),
    );

    return acceleration;
}

/** Call back with the device's orientation angles, at most once an interval, until the returned function is called. */
export function makeGyroscope(
    onChange: (orientation: GyroscopeReading) => void,
    options: { readonly interval?: number } = {},
): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }

    return makeThrottledListener(
        "deviceorientation",
        options.interval ?? READING_INTERVAL,
        (event: DeviceOrientationEvent) =>
            onChange({ alpha: event.alpha, beta: event.beta, gamma: event.gamma }),
    );
}

/** Follow the device's orientation angles, each null until reported, until cleanup. */
export function createGyroscope(interval = READING_INTERVAL): GyroscopeReading {
    // read nothing on the server
    if (isServer) {
        return { alpha: null, beta: null, gamma: null };
    }
    const [reading, setReading] = createSignal<GyroscopeReading>(
        { alpha: null, beta: null, gamma: null },
        { ownedWrite: true },
    );
    onCleanup(makeGyroscope((orientation) => setReading(orientation), { interval }));

    return {
        get alpha() {
            return reading().alpha;
        },
        get beta() {
            return reading().beta;
        },
        get gamma() {
            return reading().gamma;
        },
    };
}

/** Start the device's magnetometer, calling back with each reading in microteslas, or return null where the device has no magnetometer API. */
export function makeCompass(
    onChange: (reading: CompassReading) => void,
    options?: CompassOptions,
): (() => void) | null {
    // find the magnetometer, absent on the server and where the device has none
    const Magnetometer: unknown = isServer ? undefined : Reflect.get(globalThis, "Magnetometer");
    if (!isMagnetometerConstructor(Magnetometer)) {
        return null;
    }

    return makeSensor(
        Magnetometer,
        (sensor) => onChange({ x: sensor.x, y: sensor.y, z: sensor.z }),
        options,
    );
}

/** Follow the device's magnetometer in microteslas, each axis null until read, until cleanup. */
export function createCompass(options?: CompassOptions): CompassReading {
    // follow each reading in a signal
    const [reading, setReading] = createSignal<CompassReading>(
        { x: null, y: null, z: null },
        { ownedWrite: true },
    );
    const stop = makeCompass((next) => setReading(next), options);
    if (stop !== null) {
        onCleanup(stop);
    }

    return {
        get x() {
            return reading().x;
        },
        get y() {
            return reading().y;
        },
        get z() {
            return reading().z;
        },
    };
}

/** Call back with the battery's state once the API answers and on each change, until the returned function is called, doing nothing where the device has no Battery Status API. */
export function makeBattery(onChange: (reading: BatteryReading) => void): () => void {
    // read nothing on the server or without the API
    const getBattery: unknown = isServer ? undefined : Reflect.get(navigator, "getBattery");
    if (typeof getBattery !== "function") {
        return () => {};
    }

    // report the state and its changes once the battery answers, unless stopped before, reporting a refusal
    let isStopped = false;
    let detach: (() => void) | undefined;
    Promise.resolve(Reflect.apply(getBattery, navigator, []))
        .then((battery: unknown) => {
            // skip an answer that came after stopping, and refuse one without a battery
            if (isStopped) {
                return;
            } else if (!isBatteryManager(battery)) {
                throw new TypeError("the battery status API answered without a battery");
            }
            const update = (): void =>
                onChange({
                    charging: battery.charging,
                    chargingTime: battery.chargingTime,
                    dischargingTime: battery.dischargingTime,
                    level: battery.level,
                });
            update();
            for (const name of BATTERY_EVENTS) {
                battery.addEventListener(name, update);
            }
            detach = () => {
                for (const name of BATTERY_EVENTS) {
                    battery.removeEventListener(name, update);
                }
            };
        })
        .catch(reportError);

    return () => {
        isStopped = true;
        detach?.();
    };
}

/** Follow the battery's state, undefined until the API answers or where the device has none, until cleanup. */
export function createBattery(): Accessor<BatteryReading | undefined> {
    // read nothing on the server
    if (isServer) {
        return () => undefined;
    }
    const [reading, setReading] = createSignal<BatteryReading | undefined>(undefined, {
        ownedWrite: true,
    });
    onCleanup(makeBattery((next) => setReading(next)));

    return reading;
}

/** Listen to a window event at most once an interval, until the returned function is called. */
function makeThrottledListener<Type extends keyof WindowEventMap>(
    type: Type,
    interval: number,
    handler: (event: WindowEventMap[Type]) => void,
): () => void {
    // handle one event, then wait out the interval
    let isThrottled = false;
    const listener = (event: WindowEventMap[Type]): void => {
        if (isThrottled) {
            return;
        }
        isThrottled = true;
        setTimeout(() => {
            isThrottled = false;
        }, interval);
        handler(event);
    };
    window.addEventListener(type, listener);

    return () => window.removeEventListener(type, listener);
}

/** Check whether a global is a magnetometer constructor. */
function isMagnetometerConstructor(
    value: unknown,
): value is SensorConstructor<MagnetometerSensor, CompassOptions> {
    return typeof value === "function";
}

/** Check whether the battery API answered with a battery manager. */
function isBatteryManager(value: unknown): value is BatteryManager {
    return value instanceof EventTarget && "level" in value;
}

/** Report a sensor's error event. */
function fail(event: Event): void {
    reportError(event instanceof ErrorEvent ? event.error : event);
}
