/** The realm's own globals, saved while a simulation replaces them. */
export const SYSTEM = {
    Date: globalThis.Date,
    setTimeout: globalThis.setTimeout,
    clearTimeout: globalThis.clearTimeout,
    setInterval: globalThis.setInterval,
    clearInterval: globalThis.clearInterval,
    performanceNow: performance.now.bind(performance),
    getRandomValues: crypto.getRandomValues.bind(crypto),
    randomUUID: crypto.randomUUID.bind(crypto),
    random: Math.random.bind(Math),
    fetch: globalThis.fetch.bind(globalThis),
    WebSocket: globalThis.WebSocket,
};
