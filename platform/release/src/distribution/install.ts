import { Updater, Release } from "@destack/update";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { RepositoryConfiguration } from "../repository/index.ts";

/** Install the published native release and verify its persistent daemon and update selection. */
async function verifyInstallation(): Promise<void> {
    // authenticate downloaded executables before invoking the distribution's own installer
    const configuration = new RepositoryConfiguration();
    const directory = await mkdtemp(join(tmpdir(), "destack-published-"));
    const target = Release.target();
    const state = join(directory, "state");
    let executable: string | undefined;
    let isRunning = false;
    let nativeApplication: string | undefined;
    let failure: unknown;
    try {
        let version: string;
        let archive: string;
        {
            await using updater = await Updater.open({
                directory: join(directory, "bootstrap"),
                repository: configuration.url,
                root: JSON.stringify((await configuration.root()).toJSON()),
                target,
                application:
                    process.platform === "darwin" ? join(directory, "bootstrap.app") : undefined,
            });
            const update = await updater.check();
            if (!update) {
                throw new Error("published repository contains no installable release");
            }
            version = update.release.version;
            if (version !== process.env.DESTACK_RELEASE_VERSION) {
                throw new Error("published release differs from the workflow version");
            }
            const download = await update.download();
            archive = download.archive;
            const staged = await updater.stage(download);
            executable = join(
                staged.directory,
                "bin",
                process.platform === "win32" ? "destack.exe" : "destack",
            );
        }

        // install twice through the shipped CLI and preserve one isolated user directory
        const environment = {
            ...process.env,
            DESTACK_DIRECTORY: state,
            DESTACK_UPDATE_URL: configuration.url.href,
        };
        if (process.platform === "win32") {
            if (process.env.CI !== "true") {
                throw new Error("verify Windows Setup on a disposable CI runner");
            }
            const setup = await downloadSetup(directory, configuration, version);
            const application = join(directory, "Destack");
            await command(setup, ["/S", `/D=${application}`], environment);
            nativeApplication = application;
            await command(setup, ["/S", `/D=${nativeApplication}`], environment);
            executable = join(nativeApplication, "helpers/destack.exe");
        }
        // exercise the per-user archive installation on macOS and Linux
        else {
            await command(executable, ["install", "--archive", archive], environment);
            await command(executable, ["install", "--archive", archive], environment);
        }
        await command(executable, ["daemon", "start", "--json"], environment);
        isRunning = true;
        const before = JSON.parse(
            await command(executable, ["host", "get", "--json"], environment),
        );
        await command(executable, ["daemon", "stop", "--json"], environment);
        isRunning = false;
        await command(executable, ["daemon", "start", "--json"], environment);
        isRunning = true;
        const after = JSON.parse(await command(executable, ["host", "get", "--json"], environment));
        if (!isDeepStrictEqual(before, after)) {
            throw new Error("installed daemon did not preserve its host record across restart");
        }

        // require the installed executable to authenticate its current public update feed
        const selection = JSON.parse(
            await command(executable, ["update", "--check", "--json"], environment),
        );
        if (
            !isDeepStrictEqual(selection, { current: version, latest: version, available: false })
        ) {
            throw new Error("installed update selection differs from the published release");
        }
        console.log(`verified installation and restart of ${version} on ${target}`);
    } catch (error) {
        // preserve the original verification failure if cleanup also fails
        failure = error;
    }

    // complete cleanup before reporting either verification or cleanup failure
    try {
        // stop verified installations before deleting their temporary executable files
        if (isRunning && executable) {
            await command(executable, ["daemon", "stop", "--json"], {
                ...process.env,
                DESTACK_DIRECTORY: state,
            });
        }
        if (nativeApplication) {
            await command(
                join(nativeApplication, "Uninstall.exe"),
                ["/S", `_?=${nativeApplication}`],
                { ...process.env, DESTACK_DIRECTORY: state },
            );
        }

        // retain failed installations for runner diagnostics
        if (!failure) {
            await rm(directory, { recursive: true, force: true });
        }
    } catch (error) {
        if (failure) {
            throw new AggregateError(
                [failure, error],
                "installation verification and cleanup failed",
            );
        }
        throw error;
    }
    if (failure) {
        throw failure;
    }
}

/** Download the native Windows installer using the same embedded TUF root as the updater. */
async function downloadSetup(
    directory: string,
    configuration: RepositoryConfiguration,
    version: string,
): Promise<string> {
    // authenticate the installer independently of the archive staged by the updater
    const tuf = await import("tuf-js");
    const metadata = join(directory, "setup-metadata");
    await mkdir(metadata);
    await writeFile(
        join(metadata, "root.json"),
        JSON.stringify((await configuration.root()).toJSON()),
    );
    const updater = new tuf.Updater({
        metadataDir: metadata,
        metadataBaseUrl: new URL("metadata/", configuration.url).href,
        targetDir: join(directory, "setup"),
        targetBaseUrl: new URL("targets/", configuration.url).href,
    });
    await updater.refresh();
    const target = await updater.getTargetInfo("x86_64-pc-windows-msvc.exe");
    if (!target || target.custom.version !== version) {
        throw new Error("setup does not match the verified archive release");
    }

    return await updater.downloadTarget(target);
}

/** Execute a shipped command with bounded runtime and complete diagnostics. */
async function command(
    executable: string,
    arguments_: string[],
    environment: NodeJS.ProcessEnv,
): Promise<string> {
    // retain output for exact assertions without inheriting interactive input
    const child = Bun.spawn([executable, ...arguments_], {
        env: environment,
        stdin: "ignore",
        stdout: "pipe",
        stderr: "pipe",
        timeout: 120000,
    });
    const [code, output, diagnostic] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
        throw new Error(
            `installed command ${arguments_.join(" ")} failed (${code}): ${diagnostic}`,
        );
    }

    return output;
}

await verifyInstallation();
