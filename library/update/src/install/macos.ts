import { UpdateError } from "../error/error.ts";
import { dirname, join } from "node:path";
import { mkdir, mkdtemp, lstat, rename, rm } from "node:fs/promises";
import { dlopen } from "bun:ffi";

/** Copy a verified Mac application and atomically replace an existing installation. */
export async function installApplication(source: string, destination: string): Promise<void> {
    // stage the bundle beside the destination for atomic replacement
    await mkdir(dirname(destination), { recursive: true });
    const temporary = await mkdtemp(join(dirname(destination), ".destack-"));
    const pending = join(temporary, "Destack.app");
    try {
        const copy = Bun.spawn(["/usr/bin/ditto", source, pending], {
            stdout: "ignore",
            stderr: "pipe",
        });
        const [copyCode, copyError] = await Promise.all([
            copy.exited,
            new Response(copy.stderr).text(),
        ]);
        if (copyCode !== 0) {
            throw new UpdateError("INSTALL", copyError);
        }
        await verifyApplication(pending);

        // exchange complete bundles in one filesystem operation and retain the old release archive
        let exists = true;
        try {
            await lstat(destination);
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
                throw error;
            }
            exists = false;
        }
        if (exists) {
            await verifyApplication(destination);
            const system = dlopen("/usr/lib/libSystem.B.dylib", {
                renamex_np: { args: ["buffer", "buffer", "u32"], returns: "i32" },
            });
            const encoder = new TextEncoder();
            try {
                if (
                    system.symbols.renamex_np(
                        encoder.encode(pending + "\0"),
                        encoder.encode(destination + "\0"),
                        2,
                    ) !== 0
                ) {
                    throw new UpdateError(
                        "INSTALL",
                        `Cannot replace ${destination}. Check directory permissions.`,
                    );
                }
            } finally {
                system.close();
            }
        } else {
            await rename(pending, destination);
        }
    } finally {
        await rm(temporary, { recursive: true });
    }
}

/** Verify the complete bundle and its application identifier. */
async function verifyApplication(application: string): Promise<void> {
    const identifier = Bun.spawn(
        [
            "/usr/bin/plutil",
            "-extract",
            "CFBundleIdentifier",
            "raw",
            join(application, "Contents/Info.plist"),
        ],
        { stdout: "pipe", stderr: "inherit" },
    );
    const [identifierCode, name] = await Promise.all([
        identifier.exited,
        new Response(identifier.stdout).text(),
    ]);
    if (identifierCode !== 0 || name.trim() !== "sh.destack.desktop") {
        throw new UpdateError("INSTALL", `Not a Destack application: ${application}`);
    }
    const signature = Bun.spawn(
        ["/usr/bin/codesign", "--verify", "--deep", "--strict", application],
        {
            stdout: "ignore",
            stderr: "pipe",
        },
    );
    const [signatureCode, signatureError] = await Promise.all([
        signature.exited,
        new Response(signature.stderr).text(),
    ]);
    if (signatureCode !== 0) {
        throw new UpdateError("INSTALL", signatureError);
    }
}
