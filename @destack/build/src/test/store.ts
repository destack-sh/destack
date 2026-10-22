import { fileURLToPath } from "node:url";
import { LocalPackageStore } from "../store/local.ts";

/** Open the store tests keep builds in across runs, below the checkout and shared by processes. */
export function testStore(): Promise<LocalPackageStore> {
    return LocalPackageStore.checkout(fileURLToPath(new URL(".", import.meta.url)), "test");
}
