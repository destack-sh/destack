import { mount, StartClient } from "@solidjs/start/client";

const dispose = mount(() => <StartClient />, document.getElementById("app")!);

export default dispose;
