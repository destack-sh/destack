import { mount, StartClient } from "@solidjs/start/client";

/// The server-rendered application root.
const root = document.getElementById("app");
if (root == undefined) {
    throw new Error("application root is missing");
}

/// The active SolidStart client root.
const dispose = mount(() => <StartClient />, root);

export default dispose;
