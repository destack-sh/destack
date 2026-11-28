import manifest from "../../package.json" with { type: "json" };

/** The state units and part builders the package exports, which show no examples. */
const STATE = new Set([
    "action",
    "build",
    "collection",
    "drag",
    "focus",
    "interaction",
    "markdown",
    "part",
    "position",
    "selection",
    "toggle-state",
    "virtualizer",
]);

/** The components that render only inside a host's mounted view, which examples cannot give yet. */
const HOSTED = new Set(["chrome"]);

/** The component directories the package exports, without stylesheets, state or hosted parts. */
export const DIRECTORIES = Object.keys(manifest.exports)
    .filter((path) => !path.includes("*") && !path.endsWith(".css"))
    .map((path) => path.slice(2))
    .filter((name) => !STATE.has(name) && !HOSTED.has(name));
