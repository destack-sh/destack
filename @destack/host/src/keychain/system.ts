import type { Keychain } from "./keychain.ts";

/** The operating system's keychain, for one service's secrets. */
export class SystemKeychain implements Keychain {
    /** The keychain service, such as `app.destack.host`. */
    readonly service: string;

    /** Open a keychain service. */
    constructor(service: string) {
        this.service = service;
    }

    /** Read a secret, absent before one was kept. */
    async load(name: string): Promise<string | undefined> {
        return (await Bun.secrets.get({ service: this.service, name })) ?? undefined;
    }

    /** Keep a secret, replacing the one kept before. */
    async save(name: string, value: string): Promise<void> {
        await Bun.secrets.set({ service: this.service, name, value });
    }

    /** Forget a secret. */
    async remove(name: string): Promise<void> {
        await Bun.secrets.delete({ service: this.service, name });
    }
}
