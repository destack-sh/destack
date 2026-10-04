import { Window } from "happy-dom";

/** The DOM event globals that replace the runtime's events. */
const DOM_GLOBALS = new Set(["Event", "EventTarget", "CustomEvent", "UIEvent", "KeyboardEvent"]);

/** Render tests into a DOM, keeping the runtime's fetch, streams and timers. */
export default {
    name: "dom",
    viteEnvironment: "client",
    setup(global: Record<string, unknown>) {
        // expose the window's DOM events and the globals the runtime lacks
        const window = new Window({ url: "http://localhost/" });
        const added: string[] = [];
        const replaced = new Map<string, unknown>();
        for (const key of Object.getOwnPropertyNames(window)) {
            // keep the runtime's global
            if (!DOM_GLOBALS.has(key) && key in global) {
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
