import { UpdateError } from "../error/error.ts";
import { dirname, join } from "node:path";

/** Copy a verified Mac application and atomically replace an existing installation. */
export async function installApplication(source: string, destination: string): Promise<void> {
    // stage the bundle beside the destination for atomic replacement
    await Deno.mkdir(dirname(destination), { recursive: true });
    const temporary = await Deno.makeTempDir({ dir: dirname(destination), prefix: ".destack-" });
    const pending = join(temporary, "Destack.app");
    try {
        const copy = await new Deno.Command("/usr/bin/ditto", {
            args: [source, pending],
            stderr: "piped",
        }).output();
        if (!copy.success) {
            throw new UpdateError("INSTALL", new TextDecoder().decode(copy.stderr));
        }
        await verifyApplication(pending);

        // exchange complete bundles in one filesystem operation and retain the old release archive
        let exists = true;
        try {
            await Deno.lstat(destination);
        } catch (error) {
            if (!(error instanceof Deno.errors.NotFound)) {
                throw error;
            }
            exists = false;
        }
        if (exists) {
            await verifyApplication(destination);
            const system = Deno.dlopen("/usr/lib/libSystem.B.dylib", {
                renamex_np: { parameters: ["buffer", "buffer", "u32"], result: "i32" },
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
            await Deno.rename(pending, destination);
        }
    } finally {
        await Deno.remove(temporary, { recursive: true });
    }
}

/** Verify the complete bundle and its application identifier. */
async function verifyApplication(application: string): Promise<void> {
    const identifier = await new Deno.Command("/usr/bin/plutil", {
        args: ["-extract", "CFBundleIdentifier", "raw", join(application, "Contents/Info.plist")],
        stdout: "piped",
        stderr: "piped",
    }).output();
    if (
        !identifier.success ||
        new TextDecoder().decode(identifier.stdout).trim() !== "sh.destack.desktop"
    ) {
        throw new UpdateError("INSTALL", `Not a Destack application: ${application}`);
    }
    const signature = await new Deno.Command("/usr/bin/codesign", {
        args: ["--verify", "--deep", "--strict", application],
        stderr: "piped",
    }).output();
    if (!signature.success) {
        throw new UpdateError("INSTALL", new TextDecoder().decode(signature.stderr));
    }
}
