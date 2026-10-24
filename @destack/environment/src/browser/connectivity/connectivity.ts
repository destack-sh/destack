/** Whether a browser reaches the network, as `navigator.onLine` and its events have it. */
export interface Connectivity {
    /** Report whether the browser is online. */
    readonly isOnline: () => boolean;
    /** Follow each change of whether the browser is online, until disposed. */
    readonly on: (listener: (isOnline: boolean) => void) => Disposable;
}

/** Connectivities. */
export const Connectivity: { readonly system: Connectivity } = {
    /** The browser's connectivity. */
    system: {
        isOnline: () => navigator.onLine,
        on: (listener) => {
            // report each online and offline event with the state it brings
            const online = () => listener(true);
            const offline = () => listener(false);
            window.addEventListener("online", online);
            window.addEventListener("offline", offline);

            return {
                [Symbol.dispose]: () => {
                    window.removeEventListener("online", online);
                    window.removeEventListener("offline", offline);
                },
            };
        },
    },
};
