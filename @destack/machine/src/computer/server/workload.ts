import { ResourceHandle } from "@destack/resource";
import { defineWorkload } from "@destack/object/workload";
import { ComputerServer } from "./computer.ts";

/** The machine package's server on a computer, which the machine process binds. */
export const computerServer = new ResourceHandle<ComputerServer>(
    import.meta.destack.package,
    "computer",
);

/** The packages of the device's partition: the computer's record and local objects. */
export const deviceWorkload = defineWorkload({
    name: "device",
    start: (context) => computerServer.get(context.resources).serveDevice(),
});

/** The packages of the machine's own partition, which serves once the machine enrolls. */
export const machineWorkload = defineWorkload({
    name: "computer",
    start: (context) => computerServer.get(context.resources).serveMachine(context),
});
