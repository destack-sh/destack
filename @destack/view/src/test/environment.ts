import { Window } from "happy-dom";

/** The DOM event globals that replace the runtime's: the event target and every event class. */
const DOM_EVENT = /^(EventTarget|\w*Event)$/u;

/** Render tests into a DOM, keeping the runtime's fetch, streams and timers. */
export default {
    name: "dom",
    viteEnvironment: "client",
    setup(global: Record<string, unknown>) {
        // expose the window's DOM events and the globals the runtime lacks, giving iframes windows as browsers do
        const window = new Window({
            url: "http://localhost/",
            settings: { enableIframePageLoading: true },
        });

        // give the document the CSS Font Loading API's set of faces, empty as the page declares none
        if (!("fonts" in window.document)) {
            Object.defineProperty(window.document, "fonts", {
                value: new Set(),
                configurable: true,
            });
        }

        // copy the window's globals over the runtime's, noting what to restore
        const added: string[] = [];
        const replaced = new Map<string, unknown>();
        for (const key of Object.getOwnPropertyNames(window)) {
            // keep the runtime's global, but for events, as one event family dispatches only its own
            if (!DOM_EVENT.test(key) && key in global) {
                continue;
            }

            // remember what to restore and bind functions to the window
            if (key in global) {
                replaced.set(key, global[key]);
            } else {
                added.push(key);
            }
            const value: unknown = Reflect.get(window, key);
            const isFunction = typeof value === "function" && !/^[A-Z]/u.test(key);
            global[key] = isFunction ? value.bind(window) : value;
        }

        return {
            async teardown() {
                // restore the runtime's globals and close the window
                for (const key of added) {
                    delete global[key];
                }
                for (const [key, value] of replaced) {
                    global[key] = value;
                }
                await window.happyDOM.close();
            },
        };
    },
};
