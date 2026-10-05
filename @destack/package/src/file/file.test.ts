import { expect, test } from "@destack/test";
import { PackageError } from "../error/error.ts";
import { PackageFile } from "./file.ts";

test("verify a file's exact size and digest", async () => {
    // describe a file and accept its exact bytes
    const bytes = new TextEncoder().encode("export {};");
    const file = await PackageFile.describe("src/index.js", "text/javascript", bytes);
    await expect(PackageFile.verify(file, bytes)).resolves.toBeUndefined();

    // refuse other lengths, and other bytes of the same length
    await expect(PackageFile.verify(file, new TextEncoder().encode("export {}"))).rejects.toThrow(
        new PackageError("INVALID_FILE", "file size mismatch: src/index.js"),
    );
    await expect(PackageFile.verify(file, new TextEncoder().encode("export [];"))).rejects.toThrow(
        new PackageError("INVALID_FILE", "file digest mismatch: src/index.js"),
    );
});
