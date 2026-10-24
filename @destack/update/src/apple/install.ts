import { Uuid } from "@destack/schema";
import { type Environment, FileSystemError } from "@destack/environment";
import { UpdateError } from "../error/error.ts";
import { dirname, join } from "node:path";
import { dlopen } from "bun:ffi";

/** Copy a verified Mac application and atomically replace an existing installation. */
export async function installApplication(
    source: string,
    destination: string,
    identifier: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // run the system tools' file and process effects as host work
    await environment.clock.trackHost(
        replaceApplication(source, destination, identifier, environment),
    );
}

/** Copy a verified Mac application with the system's tools, replacing the destination. */
async function replaceApplication(
    source: string,
    destination: string,
    identifier: string,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // stage the bundle beside the destination for atomic replacement
    const { files, process: host } = environment;
    await files.mkdir(dirname(destination), { recursive: true });
    const temporary = join(dirname(destination), `.destack-${Uuid.v4(environment.random)}`);
    await files.mkdir(temporary);
    const pending = join(temporary, "Destack.app");
    try {
        const copy = host.spawn(["/usr/bin/ditto", source, pending], {
            stdout: "ignore",
            stderr: "pipe",
        });
        const [copyCode, copyError] = await Promise.all([
            copy.exited,
            new Response(copy.stderr).text(),
        ]);
        if (copyCode !== 0) {
            throw UpdateError.of("INSTALL_FAILED", copyError);
        }
        await verifyApplication(pending, identifier, environment);

        // exchange complete bundles in one filesystem operation and retain the old release archive
        const hasDestination = await files.lstat(destination).then(
            () => true,
            (error: unknown) => {
                // read a missing destination as a first installation
                if (!FileSystemError.is(error, "NOT_FOUND")) {
                    throw error;
                }

                return false;
            },
        );
        if (hasDestination) {
            await verifyApplication(destination, identifier, environment);
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
                    throw UpdateError.of(
                        "INSTALL_FAILED",
                        `cannot replace ${destination}: check directory permissions`,
                    );
                }
            } finally {
                system.close();
            }
        } else {
            await files.rename(pending, destination);
        }
    } finally {
        await files.remove(temporary, { recursive: true });
    }
}

/** Verify the complete bundle and its application identifier. */
export async function verifyApplication(
    application: string,
    expectedIdentifier: string,
    environment: Environment<"process">,
): Promise<void> {
    // read the bundle identifier
    const { process: host } = environment;
    const identifier = host.spawn(
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
    if (identifierCode !== 0 || name.trim() !== expectedIdentifier) {
        throw UpdateError.of(
            "VERIFICATION_FAILED",
            `application identifier does not match ${expectedIdentifier}: ${application}`,
        );
    }
    const signature = host.spawn(
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
        throw UpdateError.of("VERIFICATION_FAILED", signatureError);
    }
}
