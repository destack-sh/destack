import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import {
    type ConnectionType,
    createConnectivitySignal,
    createNetworkInformation,
    type EffectiveConnectionType,
    makeConnectivityListener,
    makeNetworkInformation,
    type NetworkState,
} from "./connectivity.ts";

/** Whether the stand-in browser is online. */
let isOnline = true;

/** A stand-in for the browser's connection. */
class StubConnection extends EventTarget {
    /** The bandwidth estimate. */
    downlink = 10;
    /** The highest downlink. */
    downlinkMax: number | undefined = undefined;
    /** How fast the connection effectively is. */
    effectiveType: EffectiveConnectionType = "4g";
    /** The round-trip estimate. */
    rtt = 50;
    /** Whether the person asked to save data, behind the interface's name. */
    isSavingData = false;
    /** The connection's technology. */
    type: ConnectionType = "wifi";

    /** Read whether the person asked to save data. */
    get saveData(): boolean {
        return this.isSavingData;
    }

    /** Write whether the person asked to save data. */
    set saveData(value: boolean) {
        this.isSavingData = value;
    }
}

/** The connection of the current test. */
let connection = new StubConnection();

beforeEach(() => {
    isOnline = true;
    connection = new StubConnection();
    Object.defineProperty(navigator, "onLine", { configurable: true, get: () => isOnline });
    Object.defineProperty(navigator, "connection", { configurable: true, get: () => connection });
});

afterEach(() => {
    Reflect.deleteProperty(navigator, "onLine");
    Reflect.deleteProperty(navigator, "connection");
});

/** Go online or offline, telling the window. */
function setOnline(value: boolean): void {
    isOnline = value;
    window.dispatchEvent(new Event(value ? "online" : "offline"));
}

test("call back as the browser goes offline and online, until disposed", () => {
    const seen: boolean[] = [];
    createRoot((disposeRoot) => {
        makeConnectivityListener((value) => seen.push(value));
        setOnline(false);
        setOnline(true);
        disposeRoot();
    });
    setOnline(false);

    expect(seen).toEqual([false, true]);
});

test("follow whether the browser is online", () => {
    const observed = createRoot((disposeRoot) => {
        const online = createConnectivitySignal();
        const values = [online()];
        for (const value of [false, true]) {
            setOnline(value);
            flush();
            values.push(online());
        }
        disposeRoot();

        return values;
    });

    expect(observed).toEqual([true, false, true]);
});

test("report the network's state on online changes and connection changes", () => {
    const seen: NetworkState[] = [];
    createRoot((disposeRoot) => {
        makeNetworkInformation((state) => seen.push(state));
        setOnline(false);
        Object.assign(connection, {
            effectiveType: "2g",
            rtt: 400,
            downlink: 1.5,
            saveData: true,
            type: "cellular",
        });
        connection.dispatchEvent(new Event("change"));
        disposeRoot();
    });

    expect(seen).toEqual([
        {
            online: false,
            downlink: 10,
            downlinkMax: undefined,
            effectiveType: "4g",
            rtt: 50,
            saveData: false,
            type: "wifi",
        },
        {
            online: false,
            downlink: 1.5,
            downlinkMax: undefined,
            effectiveType: "2g",
            rtt: 400,
            saveData: true,
            type: "cellular",
        },
    ]);
});

test("follow each property of the network's state", () => {
    const observed = createRoot((disposeRoot) => {
        const network = createNetworkInformation();
        const read = (): unknown[] => [
            network.online(),
            network.effectiveType(),
            network.downlink(),
            network.rtt(),
            network.saveData(),
            network.type(),
        ];
        const initial = read();
        setOnline(false);
        Object.assign(connection, { effectiveType: "3g", rtt: 200 });
        connection.dispatchEvent(new Event("change"));
        flush();
        const changed = read();
        disposeRoot();

        return [initial, changed];
    });

    expect(observed).toEqual([
        [true, "4g", 10, 50, false, "wifi"],
        [false, "3g", 10, 200, false, "wifi"],
    ]);
});
