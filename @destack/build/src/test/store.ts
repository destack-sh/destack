import { fileURLToPath } from "node:url";
import { testHost } from "@destack/test/host";
import { LocalPackageStore } from "../store/local.ts";

/** Open the store of builds tests share across runs and processes, below the checkout. */
export function testStore(): Promise<LocalPackageStore> {
    return LocalPackageStore.checkout(
        fileURLToPath(new URL(".", import.meta.url)),
        "test",
        testHost(),
    );
}
