import { PackageManifest, PackageLocation } from "./manifest.ts";
import { PackageReader } from "./reader.ts";
import { verifyFile, PackagePath } from "../file/file.ts";
import { PackageError } from "../error/index.ts";

/** HTTP access supplied by the application, including authentication. */
export interface PackageHttpOptions {
    /** Authenticated fetch implementation. */
    fetch: typeof fetch;
    /** Cancel manifest and subsequent description requests. */
    signal?: AbortSignal;
}

/** Open an immutable remote package and verify its root manifest. */
export async function openPackage(
    location: PackageLocation,
    options: PackageHttpOptions,
): Promise<PackageReader> {
    // restrict relative reads to this package's HTTP endpoint
    location = PackageLocation.parse(location);
    const base = new URL(location.url);
    if (
        !["https:", "http:"].includes(base.protocol) ||
        base.username ||
        base.password ||
        base.search ||
        base.hash
    ) {
        throw new PackageError("INVALID_FILE", "invalid package URL");
    }
    if (!base.pathname.endsWith("/")) {
        base.pathname += "/";
    }

    // authenticate and verify the root before trusting its file references
    const bytes = await readPackageFile(new URL("manifest.json", base), options);
    await verifyFile(
        {
            path: "manifest.json",
            digest: location.manifest,
            size: bytes.byteLength,
            mediaType: "application/json",
        },
        bytes,
    );
    const manifest = PackageManifest.parse(
        JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
    );

    return new PackageReader(manifest, async (path) => {
        const encoded = PackagePath.parse(path).split("/").map(encodeURIComponent).join("/");

        return readPackageFile(new URL(`files/${encoded}`, base), options);
    });
}

/** Read one complete description and reject HTTP failures before decoding it. */
async function readPackageFile(
    url: URL,
    options: PackageHttpOptions,
): Promise<Uint8Array<ArrayBuffer>> {
    const response = await options.fetch(url, { signal: options.signal, redirect: "error" });
    if (!response.ok) {
        await response.body?.cancel();
        throw new PackageError("INVALID_FILE", `package read failed: HTTP ${response.status}`);
    }

    return new Uint8Array(await response.arrayBuffer());
}
