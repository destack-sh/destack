import type { Environment } from "@destack/environment";
import { Device, type MachineCredential } from "@destack/machine/device";

/** This computer's native device, opened on first use and closed with the command line. */
export class NativeDevice implements AsyncDisposable {
    /** The environment the device opens in. */
    readonly #environment: Environment<"files" | "process" | "host" | "server">;
    /** The opening in progress or done, absent before the first use. */
    #opening: Promise<Device<MachineCredential>> | undefined;
    /** The device once opened, absent before or after a failed open. */
    #device: Device<MachineCredential> | undefined;

    /** Prepare to open this computer's device in an environment. */
    constructor(environment: Environment<"files" | "process" | "host" | "server">) {
        this.#environment = environment;
    }

    /** Open the device once, keeping it to close. */
    open(): Promise<Device<MachineCredential>> {
        this.#opening ??= Device.current(this.#environment).then((device) => {
            this.#device = device;

            return device;
        });

        return this.#opening;
    }

    /** Close the device if it opened. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#device?.[Symbol.asyncDispose]();
    }
}
