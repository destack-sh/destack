import { cp, mkdir, mkdtemp, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Release, type Target } from "@destack/update/release";
import workspace from "../../../../package.json" with { type: "json" };
import solidPlugin from "@opentui/solid/bun-plugin";
import { buildExecutable } from "./index.ts";
import { run } from "./command.ts";
import { MacSigning } from "../signing/apple.ts";
import { version } from "./index.ts";

/** Repository containing all distribution entrypoints. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));
/** Bun executable targets corresponding to distribution targets. */
const TARGETS: Record<Target, Bun.Build.CompileTarget> = {
    "aarch64-apple-darwin": "bun-darwin-arm64",
    "x86_64-apple-darwin": "bun-darwin-x64",
    "aarch64-unknown-linux-gnu": "bun-linux-arm64",
    "x86_64-unknown-linux-gnu": "bun-linux-x64",
    "x86_64-pc-windows-msvc": "bun-windows-x64",
};

/** Build the native window, TypeScript host, and CLI as one distribution. */
async function build(): Promise<void> {
    // compile distributions with the pinned runtime
    if (`bun@${Bun.version}` !== workspace.packageManager) {
        throw new Error(`release builds require ${workspace.packageManager}`);
    }

    // build each native desktop on its matching operating system
    const release = new Release(version, process.argv[2] ?? Release.target());
    const target = release.target;
    const identity = process.env.DESTACK_RELEASE_CHANNEL ?? "dev";
    if (identity !== "stable" && identity !== "nightly" && identity !== "dev") {
        throw new Error("select stable, nightly or dev application identity");
    }
    const isMac = target.includes("apple");
    const isWindows = target.includes("windows");
    if (isMac !== (process.platform === "darwin") || isWindows !== (process.platform === "win32")) {
        throw new Error("build desktop distributions on their target operating system");
    }

    // retain completed distributions while producing their replacements
    const output = join(ROOT, "dist", release.version);
    const staging = join(ROOT, "dist/.build");
    await mkdir(output, { recursive: true });
    await mkdir(staging, { recursive: true });
    const directory = await mkdtemp(join(staging, `${target}-`));
    const view = join(directory, "view");
    await run(
        process.execPath,
        ["run", "platform/release/src/distribution/frontend.ts", view],
        ROOT,
    );

    // compile the CLI and desktop host from the installed workspace graph
    const cli = join(directory, "bin", isWindows ? "destack.exe" : "destack");
    const daemon = join(directory, "bin", isWindows ? "destack-daemon.exe" : "destack-daemon");
    const sandbox = join(directory, "bin", isWindows ? "destack-sandbox.exe" : "destack-sandbox");
    const host = join(directory, isWindows ? "destack-desktop-host.exe" : "destack-desktop-host");
    await mkdir(dirname(cli), { recursive: true });
    for (const [entrypoint, outfile] of [
        ["@destack/cli/src/main.ts", cli],
        ["@destack/daemon/src/main.ts", daemon],
        ["@destack/sandbox/src/main.ts", sandbox],
        ["@destack/desktop/src/main.ts", host],
    ]) {
        await buildExecutable({
            root: ROOT,
            entrypoint: join(ROOT, entrypoint),
            outfile,
            target,
            runtime: TARGETS[target],
            version,
            identity,
            plugins: outfile === cli ? [solidPlugin] : [],
        });
    }

    // use Tauri's native bundler for the platform window
    const native = join(ROOT, "@destack/desktop/native");
    const nativeOutput = join(ROOT, "dist/.native");
    const tauri = join(ROOT, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    // derive the native application identity and artwork from the selected distribution
    const suffix = identity === "stable" ? "" : `-${identity}`;
    const title =
        identity === "stable"
            ? "Destack"
            : identity === "nightly"
              ? "Destack Nightly"
              : "Destack Dev";
    const icons = join(directory, "icons");
    await mkdir(icons);
    let artwork = join(ROOT, `platform/brand/icon/icon${suffix}.svg`);
    if (isMac) {
        await run(
            "swift",
            [
                "-module-cache-path",
                join(staging, "swift"),
                "platform/release/src/icon/macos.swift",
                `platform/brand/icon/icon${suffix}.png`,
                join(icons, "source.png"),
            ],
            ROOT,
        );

        artwork = join(icons, "source.png");
    }
    await run(process.execPath, ["run", tauri, "icon", artwork, "--output", icons], native);

    // pass generated configuration without modifying native source files
    const configuration = JSON.stringify({
        productName: title,
        version,
        identifier: `sh.destack.desktop${identity === "stable" ? "" : `.${identity}`}`,
        bundle: { icon: [join(icons, "icon.png"), join(icons, isMac ? "icon.icns" : "icon.ico")] },
    });
    const arguments_ = ["run", tauri, "build", "--target", target, "--config", configuration];
    if (!isMac) {
        arguments_.push("--no-bundle");
    }
    await run(process.execPath, arguments_, native, {
        CARGO_TARGET_DIR: nativeOutput,
        DESTACK_RELEASE_CHANNEL: identity,
    });
    const compiled = join(nativeOutput, target, "release");
    const application = join(directory, isMac ? "Destack.app" : "Destack");
    if (isMac) {
        await cp(join(compiled, `bundle/macos/${title}.app`), application, { recursive: true });
    } else {
        await mkdir(application);
        await cp(
            join(compiled, isWindows ? "Destack.exe" : "Destack"),
            join(application, isWindows ? "Destack.exe" : "Destack"),
        );
    }
    await rm(icons, { recursive: true });

    // include Linux menu artwork and a direct installer beside the extracted application
    if (target.endsWith("unknown-linux-gnu")) {
        await cp(
            join(ROOT, `platform/brand/icon/icon${suffix}.svg`),
            join(application, "icon.svg"),
        );
        await writeFile(
            join(application, "install.sh"),
            '#!/bin/sh\nset -eu\napplication=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\n"$application/helpers/destack" install\n"$application/helpers/destack" desktop\n',
            { mode: 0o755 },
        );
    }

    // retain complete Bun executables beside the native desktop
    const commands = join(application, isMac ? "Contents/Helpers" : "helpers");
    await mkdir(commands, { recursive: true });
    await rename(view, join(application, isMac ? "Contents/view" : "view"));
    await cp(cli, join(commands, isWindows ? "destack.exe" : "destack"));
    await cp(daemon, join(commands, isWindows ? "destack-daemon.exe" : "destack-daemon"));
    await cp(sandbox, join(commands, isWindows ? "destack-sandbox.exe" : "destack-sandbox"));
    await rename(
        host,
        join(commands, isWindows ? "destack-desktop-host.exe" : "destack-desktop-host"),
    );
    if (isMac) {
        const signing = new MacSigning();
        await signing.application(application);
        for (const name of ["destack", "destack-daemon", "destack-sandbox"]) {
            await cp(join(commands, name), join(directory, "bin", name));
        }
        await signing.notarizeApplication(application);
    }

    // publish the complete directory without deleting the previous build
    const destination = join(output, target);
    try {
        await rename(destination, `${directory}.previous`);
    } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
            throw error;
        }
    }
    await rename(directory, destination);
    console.log(destination);
}

await build();
