import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Environment } from "@destack/environment";
import { run } from "../distribution/command.ts";
import { APPLE_BUNDLE } from "../distribution/platform.ts";
import type { Signer } from "./signer.ts";
import { Distribution } from "@destack/update";

/** The first bytes of 64-bit and universal Mach-O files (mach-o/loader.h, mach-o/fat.h). */
const MACH_O_MAGICS: readonly (readonly number[])[] = [
    [0xcf, 0xfa, 0xed, 0xfe],
    [0xca, 0xfe, 0xba, 0xbe],
];

/** The length of a Mach-O file's magic number, in bytes. */
const MACH_O_MAGIC_LENGTH = 4;

/** Apple code signing and notarization selected for a release build. */
export class MacSigning implements Signer {
    /** The keychain identity, or '-' for a local development build. */
    readonly identity: string;
    /** Keychain profile configured for notarization of public releases. */
    readonly profile: string | undefined;
    /** Temporary keychain used by CI, or the user's configured search list. */
    readonly keychain: string | undefined;
    /** The host the signing tools run on. */
    readonly environment: Environment<"files" | "process">;

    /** Require production credentials when release signing is selected. */
    constructor(environment: Environment<"files" | "process">) {
        // distinguish explicit local signing from required production credentials
        const { variables } = environment.process;
        const mode = variables["DESTACK_SIGNING"] ?? "development";
        if (mode !== "development" && mode !== "release") {
            throw new Error("DESTACK_SIGNING must be development or release");
        }
        this.identity = mode === "release" ? required(variables, "APPLE_SIGNING_IDENTITY") : "-";
        this.profile = mode === "release" ? required(variables, "APPLE_NOTARY_PROFILE") : undefined;
        const keychain = variables["APPLE_KEYCHAIN"];
        this.keychain = keychain === "" ? undefined : keychain;
        this.environment = environment;
        if (mode === "release" && this.identity === "-") {
            throw new Error("release signing requires a Developer ID Application identity");
        }
    }

    /** Sign each nested executable and native package before signing its containing application. */
    async sign(application: string): Promise<void> {
        // sign the commands, the native code of the packages they load and the desktop before the outer bundle
        const helpers = join(application, APPLE_BUNDLE.helpers);
        await this.executables(helpers, true);
        const resources = Distribution.resources(join(helpers, "destack"));
        await this.signNative(join(resources, "node_modules"));
        await this.executables(join(application, dirname(APPLE_BUNDLE.executable)), false);
        await this.signFile(application, false);
    }

    /** Sign the native code below a directory: a toolchain's, or the packages the programs load. */
    async signNative(directory: string): Promise<void> {
        for (const path of await this.#machO(directory)) {
            await this.signFile(path, true);
        }
    }

    /** List the Mach-O files below a directory, which code signing signs one by one. */
    async #machO(directory: string): Promise<string[]> {
        // read each file's first four bytes: a thin 64-bit or a universal Mach-O header
        const { files } = this.environment;
        const entries = await files.readDirectory(directory, { recursive: true });
        const native: string[] = [];
        for (const entry of entries.filter((file) => file.kind === "file")) {
            const path = join(directory, entry.name);
            await using file = await files.open(path, { read: true });
            const header = new Uint8Array(MACH_O_MAGIC_LENGTH);
            await file.read(header);
            if (
                MACH_O_MAGICS.some((magic) => magic.every((byte, index) => header[index] === byte))
            ) {
                native.push(path);
            }
        }

        return native;
    }

    /** Verify the application's signatures and every nested signature. */
    async verify(application: string): Promise<void> {
        await run("codesign", ["--verify", "--deep", "--strict", application], this.environment);
    }

    /** Sign all executables in the selected application directory. */
    async executables(directory: string, runtime: boolean): Promise<void> {
        for (const entry of await this.environment.files.readDirectory(directory)) {
            const path = join(directory, entry.name);
            if (entry.kind === "directory") {
                await this.executables(path, runtime);
            } else if (entry.kind === "file") {
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
        const { environment } = this;
        const directory = await environment.files.makeTemporaryDirectory("destack-notarize-");
        try {
            const archive = join(directory, "application.zip");
            await run("ditto", ["-c", "-k", "--keepParent", path, archive], environment);
            await this.submit(archive);
            await this.staple(path);
            await run("spctl", ["--assess", "--type", "execute", "--verbose", path], environment);
        } finally {
            await environment.files.remove(directory, { recursive: true, force: true });
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
        await run("codesign", [...arguments_, path], this.environment);
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
        await run("xcrun", arguments_, this.environment);
    }

    /** Attach and validate the notarization ticket for offline first launch. */
    async staple(path: string): Promise<void> {
        await run("xcrun", ["stapler", "staple", path], this.environment);
        await run("xcrun", ["stapler", "validate", path], this.environment);
    }
}

/** Require a signing credential without printing its value. */
function required(variables: Readonly<Record<string, string>>, name: string): string {
    const value = variables[name];
    if (value === undefined || value === "") {
        throw new Error(`${name} is required for release signing`);
    }

    return value;
}
