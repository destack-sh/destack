import { isServer } from "@solidjs/web";
import { type Accessor, createSignal } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { createHydratableSingletonRoot } from "./rootless.ts";
import { createHydratableSignal } from "./utils.ts";

/** How fast a connection effectively is. */
export type EffectiveConnectionType = "slow-2g" | "2g" | "3g" | "4g";

/** The technology a connection runs on. */
export type ConnectionType =
    | "bluetooth"
    | "cellular"
    | "ethernet"
    | "none"
    | "wifi"
    | "wimax"
    | "other"
    | "unknown";

/** The network's state: whether the browser is online, and the connection's quality where the browser reports it. */
export type NetworkState = {
    /** Whether the browser is online. */
    readonly online: boolean;
    /** The bandwidth estimate in megabits a second. */
    readonly downlink: number | undefined;
    /** The highest downlink in megabits a second. */
    readonly downlinkMax: number | undefined;
    /** How fast the connection effectively is. */
    readonly effectiveType: EffectiveConnectionType | undefined;
    /** The round-trip estimate in milliseconds. */
    readonly rtt: number | undefined;
    /** Whether the person asked to save data. */
    readonly saveData: boolean | undefined;
    /** The connection's technology. */
    readonly type: ConnectionType | undefined;
};

/** The network's state, each property a signal of its own. */
export type NetworkInformationReturn = {
    readonly [Key in keyof NetworkState]: Accessor<NetworkState[Key]>;
};

/** The Network Information API's connection, which the DOM types leave out. */
export interface NetworkInformationConnection extends EventTarget {
    /** The bandwidth estimate in megabits a second. */
    readonly downlink: number;
    /** The highest downlink in megabits a second. */
    readonly downlinkMax?: number;
    /** How fast the connection effectively is. */
    readonly effectiveType: EffectiveConnectionType;
    /** The round-trip estimate in milliseconds. */
    readonly rtt: number;
    /** Whether the person asked to save data. */
    readonly saveData: boolean;
    /** The connection's technology. */
    readonly type?: ConnectionType;
}

/** The network on the server: online, quality unknown. */
const SERVER_NETWORK: NetworkInformationReturn = {
    online: () => true,
    downlink: () => undefined,
    downlinkMax: () => undefined,
    effectiveType: () => undefined,
    rtt: () => undefined,
    saveData: () => undefined,
    type: () => undefined,
};

/** Call back as the browser goes online and offline, until cleanup or the returned function. */
export function makeConnectivityListener(callback: (isOnline: boolean) => void): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }
    const stopOnline = makeEventListener(window, "online", () => callback(true));
    const stopOffline = makeEventListener(window, "offline", () => callback(false));

    return () => {
        stopOnline();
        stopOffline();
    };
}

/** Call back with the network's state as it goes online or offline or its connection changes. */
export function makeNetworkInformation(callback: (state: NetworkState) => void): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }

    // report on online and offline, and on connection changes where the browser has a connection
    const report = (): void => callback(readNetworkState());
    const connection = connectionOf();
    const stops = [
        makeEventListener(window, "online", report),
        makeEventListener(window, "offline", report),
        ...(connection === undefined ? [] : [makeEventListener(connection, "change", report)]),
    ];

    return () => {
        for (const stop of stops) {
            stop();
        }
    };
}

/** Follow whether the browser is online. */
export function createConnectivitySignal(): Accessor<boolean> {
    // read online on the server
    if (isServer) {
        return () => true;
    }
    const [isOnline, setIsOnline] = createHydratableSignal(true, () => navigator.onLine);
    makeConnectivityListener(setIsOnline);

    return isOnline;
}

/** Follow the network's state, each property in a signal of its own. */
export function createNetworkInformation(): NetworkInformationReturn {
    // read the server's network on the server
    if (isServer) {
        return SERVER_NETWORK;
    }

    // hold each property, updating them all on each report
    const initial = readNetworkState();
    const [online, setOnline] = createSignal(initial.online, { ownedWrite: true });
    const [downlink, setDownlink] = createSignal(initial.downlink, { ownedWrite: true });
    const [downlinkMax, setDownlinkMax] = createSignal(initial.downlinkMax, { ownedWrite: true });
    const [effectiveType, setEffectiveType] = createSignal(initial.effectiveType, {
        ownedWrite: true,
    });
    const [rtt, setRtt] = createSignal(initial.rtt, { ownedWrite: true });
    const [saveData, setSaveData] = createSignal(initial.saveData, { ownedWrite: true });
    const [type, setType] = createSignal(initial.type, { ownedWrite: true });
    makeNetworkInformation((state) => {
        // update every property from each report
        setOnline(state.online);
        setDownlink(state.downlink);
        setDownlinkMax(state.downlinkMax);
        setEffectiveType(state.effectiveType);
        setRtt(state.rtt);
        setSaveData(state.saveData);
        setType(state.type);
    });

    return { online, downlink, downlinkMax, effectiveType, rtt, saveData, type };
}

/** Follow whether the browser is online through one listener shared by every user. */
export const useConnectivitySignal: () => Accessor<boolean> =
    createHydratableSingletonRoot(createConnectivitySignal);

/** Follow the network's state through one set of listeners shared by every user. */
export const useNetworkInformation: () => NetworkInformationReturn =
    createHydratableSingletonRoot(createNetworkInformation);

/** Read the network's state now. */
function readNetworkState(): NetworkState {
    const connection = connectionOf();

    return {
        online: navigator.onLine,
        downlink: connection?.downlink,
        downlinkMax: connection?.downlinkMax,
        effectiveType: connection?.effectiveType,
        rtt: connection?.rtt,
        saveData: connection?.saveData,
        type: connection?.type,
    };
}

/** Read the browser's connection, absent where the browser has no Network Information API. */
function connectionOf(): NetworkInformationConnection | undefined {
    const connection: unknown = Reflect.get(navigator, "connection");

    return isConnection(connection) ? connection : undefined;
}

/** Check whether a value is a Network Information connection. */
function isConnection(value: unknown): value is NetworkInformationConnection {
    return value instanceof EventTarget && "effectiveType" in value;
}
