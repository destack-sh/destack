import { readdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { run } from "../distribution/command.ts";

/** Apple code signing and notarization selected for a release build. */
export class MacSigning {
    /** Keychain identity; '-' selects an explicitly local development build. */
    readonly identity: string;
    /** Keychain profile configured for notarization of public releases. */
    readonly profile?: string;
    /** Temporary keychain used by CI, or the user's configured search list. */
    readonly keychain?: string;

    /** Require production credentials when release signing is selected. */
    constructor(environment = process.env) {
        // distinguish explicit local signing from required production credentials
        const mode = environment.DESTACK_SIGNING ?? "development";
        if (mode !== "development" && mode !== "release") {
            throw new Error("DESTACK_SIGNING must be development or release");
        }
        this.identity = mode === "release" ? required(environment, "APPLE_SIGNING_IDENTITY") : "-";
        this.profile =
            mode === "release" ? required(environment, "APPLE_NOTARY_PROFILE") : undefined;
        this.keychain = environment.APPLE_KEYCHAIN;
        if (mode === "release" && this.identity === "-") {
            throw new Error("release signing requires a Developer ID Application identity");
        }
    }

    /** Sign each nested executable before signing its containing application. */
    async application(path: string): Promise<void> {
        // sign executable payloads before the outer bundle
        await this.executables(join(path, "Contents/Helpers"), true);
        await this.executables(join(path, "Contents/MacOS"), false);
        await this.sign(path, false);
        await run("codesign", ["--verify", "--deep", "--strict", path]);
    }

    /** Sign all executables in the selected application directory. */
    async executables(directory: string, runtime: boolean): Promise<void> {
        for (const entry of await readdir(directory, { withFileTypes: true })) {
            const path = join(directory, entry.name);
            if (entry.isDirectory()) {
                await this.executables(path, runtime);
            } else if (entry.isFile()) {
                await this.sign(path, runtime);
            } else {
                throw new Error(`unsupported signing entry: ${path}`);
            }
        }
    }

    /** Notarize an application and staple its ticket before packaging the update archive. */
    async notarizeApplication(path: string): Promise<void> {
        if (!this.profile) {
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
        await this.sign(path, false);
        if (this.profile) {
            await this.submit(path);
            await this.staple(path);
        }
    }

    /** Sign one exact file without recursively rewriting nested signatures. */
    async sign(path: string, runtime: boolean): Promise<void> {
        // construct the platform signing command from verified configuration
        const arguments_ = ["--force", "--sign", this.identity];
        if (this.keychain) {
            arguments_.push("--keychain", this.keychain);
        }
        if (this.profile) {
            arguments_.push("--timestamp", "--options", "runtime");
        }
        if (runtime) {
            arguments_.push("--entitlements", fileURLToPath(new URL("bun.plist", import.meta.url)));
        }
        await run("codesign", [...arguments_, path]);
    }

    /** Wait for Apple's verdict and fail the build on rejection. */
    async submit(path: string): Promise<void> {
        if (!this.profile) {
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
        if (this.keychain) {
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
    if (!value) {
        throw new Error(`${name} is required for release signing`);
    }

    return value;
}
