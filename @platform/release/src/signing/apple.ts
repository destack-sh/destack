import { readdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { run } from "../distribution/command.ts";
import { APPLE_BUNDLE } from "../distribution/platform.ts";
import type { Signer } from "./signer.ts";

/** Apple code signing and notarization selected for a release build. */
export class MacSigning implements Signer {
    /** Keychain identity; '-' selects an explicitly local development build. */
    readonly identity: string;
    /** Keychain profile configured for notarization of public releases. */
    readonly profile: string | undefined;
    /** Temporary keychain used by CI, or the user's configured search list. */
    readonly keychain: string | undefined;

    /** Require production credentials when release signing is selected. */
    constructor(environment = process.env) {
        // distinguish explicit local signing from required production credentials
        const mode = environment["DESTACK_SIGNING"] ?? "development";
        if (mode !== "development" && mode !== "release") {
            throw new Error("DESTACK_SIGNING must be development or release");
        }
        this.identity = mode === "release" ? required(environment, "APPLE_SIGNING_IDENTITY") : "-";
        this.profile =
            mode === "release" ? required(environment, "APPLE_NOTARY_PROFILE") : undefined;
        const keychain = environment["APPLE_KEYCHAIN"];
        this.keychain = keychain === "" ? undefined : keychain;
        if (mode === "release" && this.identity === "-") {
            throw new Error("release signing requires a Developer ID Application identity");
        }
    }

    /** Sign each nested executable before signing its containing application. */
    async sign(application: string): Promise<void> {
        // sign executable payloads before the outer bundle
        await this.executables(join(application, APPLE_BUNDLE.helpers), true);
        await this.executables(join(application, dirname(APPLE_BUNDLE.executable)), false);
        await this.signFile(application, false);
    }

    /** Verify the application's signatures and every nested signature. */
    async verify(application: string): Promise<void> {
        await run("codesign", ["--verify", "--deep", "--strict", application]);
    }

    /** Sign all executables in the selected application directory. */
    async executables(directory: string, runtime: boolean): Promise<void> {
        for (const entry of await readdir(directory, { withFileTypes: true })) {
            const path = join(directory, entry.name);
            if (entry.isDirectory()) {
                await this.executables(path, runtime);
            } else if (entry.isFile()) {
                await this.signFile(path, runtime);
            } else {
                throw new Error(`unsupported signing entry: ${path}`);
            }
        }
    }

    /** Notarize an application and staple its ticket before packaging the update archive. */
    async notarize(path: string): Promise<void> {
        if (this.profile === undefined) {
            return;
        }
        const directory = await mkdtemp(join(tmpdir(), "destack-notarize-"));
        try {
            const archive = join(directory, "application.zip");
            await run("ditto", ["-c", "-k", "--keepParent", path, archive]);
            await this.submit(archive);
            await this.staple(path);
            await run("spctl", ["--assess", "--type", "execute", "--verbose", path]);
        } finally {
            await rm(directory, { recursive: true, force: true });
        }
    }

    /** Sign, notarize and staple the final disk image. */
    async image(path: string): Promise<void> {
        await this.signFile(path, false);
        if (this.profile !== undefined) {
            await this.submit(path);
            await this.staple(path);
        }
    }

    /** Sign one exact file without recursively rewriting nested signatures. */
    async signFile(path: string, runtime: boolean): Promise<void> {
        // construct the platform signing command from verified configuration
        const arguments_ = ["--force", "--sign", this.identity];
        // use the keychain search list to resolve Apple's complete certificate chain
        if (this.profile !== undefined) {
            arguments_.push("--timestamp", "--options", "runtime");
        }
        if (runtime) {
            arguments_.push("--entitlements", fileURLToPath(new URL("bun.plist", import.meta.url)));
        }
        await run("codesign", [...arguments_, path]);
    }

    /** Wait for Apple's verdict and fail the build on rejection. */
    async submit(path: string): Promise<void> {
        if (this.profile === undefined) {
            throw new Error("notarization profile is missing");
        }
        const arguments_ = [
            "notarytool",
            "submit",
            path,
            "--keychain-profile",
            this.profile,
            "--wait",
            "--timeout",
            "20m",
        ];
        if (this.keychain !== undefined) {
            arguments_.push("--keychain", this.keychain);
        }
        await run("xcrun", arguments_);
    }

    /** Attach and validate the notarization ticket for offline first launch. */
    async staple(path: string): Promise<void> {
        await run("xcrun", ["stapler", "staple", path]);
        await run("xcrun", ["stapler", "validate", path]);
    }
}

/** Require a signing credential without printing its value. */
function required(environment: NodeJS.ProcessEnv, name: string): string {
    const value = environment[name];
    if (value === undefined || value === "") {
        throw new Error(`${name} is required for release signing`);
    }

    return value;
}
