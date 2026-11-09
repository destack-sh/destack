import { join } from "node:path";
import { MACHINE_ENTRY, Application } from "@destack/desktop/workload";
import { Autostart, MachineProcess, StageHome } from "@destack/machine/computer";
import { Release } from "@destack/update/release";
import { RequestId } from "@destack/sync";
import { present } from "@destack/schema";
import {
    Distribution,
    type InstalledRelease,
    Installer,
    type StagedRelease,
    UpdateError,
    type Updater,
} from "@destack/update";
import { verifyApplication } from "@destack/update/apple";
import type { DistributionRow } from "@destack/update/object";
import updatePackage from "@destack/update/package";
import { Terminal } from "../output/index.ts";
import type { CliEnvironment } from "../context/context.ts";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";

// TODO #Incomplete: measure a cold first start on Windows and state it as this bound's source
/** How long the machine's first start may take on Windows, in milliseconds: a cold start. */
const START_TIMEOUT_MILLISECONDS = 15_000;

// TODO #Incomplete: measure a restart with the device database's migration and state it as this bound's source
/** How long the machine takes to come back in the activated release, in milliseconds. */
const RESTART_TIMEOUT_MILLISECONDS = 60_000;

/** How often to ask whether the machine came back, in milliseconds, one loopback call each. */
const POLL_MILLISECONDS = 250;

/** This computer's Destack setup in a stage's home: its machine, OS supervision and desktop. */
export class Setup {
    /** The stage's home the setup installs in, with the machine's state. */
    readonly home: StageHome;
    /** The machine process of the home's state directory. */
    readonly machine: MachineProcess;
    /** The machine's supervision by the OS at login. */
    readonly autostart: Autostart;
    /** The desktop of the home. */
    readonly desktop: Application;
    /** The environment the setup installs on. */
    readonly environment: CliEnvironment;
    /** The terminal the setup reports to. */
    readonly terminal: Terminal;

    /** Take the setup of a stage's home in an environment, with its supervision and desktop. */
    constructor(fields: {
        readonly home: StageHome;
        readonly autostart: Autostart;
        readonly desktop: Application;
        readonly environment: CliEnvironment;
    }) {
        // set the stage's home, the supervision, the desktop, the environment and its terminal
        this.home = fields.home;
        this.machine = new MachineProcess(fields.home.directory, fields.environment);
        this.autostart = fields.autostart;
        this.desktop = fields.desktop;
        this.environment = fields.environment;
        this.terminal = new Terminal(fields.environment.process, fields.environment.timeZone);
    }

    /** Read the setup of this stage in an environment. */
    static async current(environment: CliEnvironment): Promise<Setup> {
        const home = StageHome.current(environment);
        const autostart = await Autostart.of(home.directory, environment);

        return new Setup({
            home,
            autostart,
            desktop: new Application(home.directory, environment),
            environment,
        });
    }

    /** Install the signed distribution and register a stable command in the user's directory. */
    static async install(environment: CliEnvironment, source?: string): Promise<void> {
        // require the compiled executable, and the desktop's libraries on Linux
        if (!environment.process.runtime.isStandalone) {
            throw CliError.of(
                "INSTALLATION_MISMATCH",
                "install using the compiled Destack executable",
            );
        }
        const { os } = environment.host;
        if (os === "linux") {
            await Distribution.requireDesktop(environment.process);
        }

        // find a native package's installed payload, else the Mac application this executable runs from
        const setup = await Setup.current(environment);
        const isDownloaded = source !== undefined;
        const native =
            !isDownloaded && os !== "darwin" ? await Distribution.native(environment) : undefined;
        const application =
            !isDownloaded && os === "darwin"
                ? Distribution.bundle(environment.process.executable)
                : undefined;

        // register a native package from its payload, without consulting the update feed
        if (native !== undefined) {
            await using _lock = await setup.installer().lock();
            const executable = await setup.distribution().link(native, { isShell: true });
            await setup.supervise(present(executable, "the native package's executable"));
        }
        // refuse Windows without its native package
        else if (os === "win32") {
            throw CliError.of("INSTALLATION_MISMATCH", "install Destack using Setup.exe");
        }
        // register the Mac application, under the installation lock
        else if (application !== undefined) {
            await using _lock = await setup.installer().lock();
            await setup.#registerApplication(application);
        }
        // install the latest signed release, copying a downloaded one's files
        else {
            await setup.#installRelease(source);
        }

        // report the installation, and the new terminal an archive's command needs
        const isArchive = native === undefined && application === undefined;
        await setup.terminal.print("Destack installed.");
        if (isArchive && !setup.home.isCustom) {
            await setup.terminal.print("Open a new terminal to use the destack command.");
        }
    }

    /** The stage's installed distribution in the home. */
    distribution(): Distribution {
        return this.home.distribution(this.environment).distribution;
    }

    /** Find the distribution files of the home, apart from the device's user databases. */
    installer(): Installer {
        return new Installer(join(this.home.directory, "distribution"), this.environment);
    }

    /** Read the release version the running machine runs, absent when no machine answers. */
    async version(): Promise<string | undefined> {
        const computer = await this.machine.loopback.computer().catch((error: unknown) => {
            if (!MachineProcess.isUnavailable(error)) {
                throw error;
            }

            return undefined;
        });

        return computer?.version;
    }

    /** Stage the stage's latest release through this computer's distribution. */
    async update(options: { readonly isStaging: boolean; readonly json: boolean }): Promise<void> {
        // find this computer's distribution, which development builds have none of
        const { scope: deviceId } = await this.machine.loopback.computer();
        const client = this.machine.loopback.connect(updatePackage);
        const [row, other] = (await client.distribution.list({ deviceId })).items;
        if (row === undefined) {
            throw CliError.of(
                "INSTALLED_DISTRIBUTION_MISSING",
                "this computer has no installed distribution",
            );
        } else if (other !== undefined) {
            throw CliError.of(
                "INSTALLATION_MISMATCH",
                "this computer records several distributions: reinstall Destack",
            );
        }

        // stage the latest release, reporting a current or staged one
        const call = () => ({
            deviceId,
            requestId: RequestId.create(this.environment),
            id: row.id,
        });
        const staged = await client.distribution.stage(call());
        if (staged.staged === null || options.isStaging) {
            await this.terminal.print(options.json ? JSON.stringify(staged) : render(staged));

            return;
        }

        // restart into it, and wait for the machine running it
        await client.distribution.activate(call());
        await this.#waitRestart(staged.staged);
        await this.terminal.print(
            options.json
                ? JSON.stringify({ version: staged.staged })
                : `Updated Destack to ${staged.staged}.`,
        );
    }

    /** Activate the staged release of this stage's distribution. */
    async activate(): Promise<void> {
        // open the installed distribution and require a staged release
        const distribution = this.distribution();
        await using updater = await distribution.open();
        const staged = await updater.staged();
        if (staged === undefined) {
            throw UpdateError.of("RELEASE_NOT_FOUND", "no staged Destack release is available");
        }

        // restart into it, and register the Mac application at its location
        const application = await distribution.application();
        const installed = await this.restart(updater, staged, application);
        if (application !== undefined) {
            await this.#registerApplication(application);
        }
        await this.terminal.print(`Updated Destack to ${installed.release.version}.`);
    }

    /** Activate a staged release, stopping the programs and starting them again in it. */
    async restart(
        updater: Pick<Updater, "current" | "activate">,
        staged: StagedRelease,
        application: string | undefined,
    ): Promise<InstalledRelease> {
        // stop the supervision and the machine, and wait for the desktop to quit with it
        const previous = await updater.current();
        const isDesktop = await this.desktop.isRunning();
        if (await this.autostart.isRegistered()) {
            await this.autostart.stop();
        }
        const isRunning = await this.stop();
        if (isDesktop) {
            await this.desktop.untilQuit();
        }

        // switch releases, leaving the programs stopped when the files may disagree with their record
        let failure: { readonly error: unknown } | undefined;
        let installed: InstalledRelease | undefined;
        try {
            installed = await updater.activate(staged);
        } catch (error) {
            if (UpdateError.is(error, "ACTIVATION_FAILED")) {
                throw error;
            }
            failure = { error };
        }

        // start the selected release's programs again, without rolling back database migrations
        const selected = (await updater.current()) ?? previous;
        try {
            await this.#resume(isRunning, selected, application);
            if (isDesktop) {
                await this.desktop.launch(
                    application ??
                        (selected === undefined
                            ? this.desktop.locate()
                            : this.desktop.locateIn(selected.directory)),
                    null,
                );
            }
        } catch (error) {
            if (failure !== undefined) {
                throw new AggregateError([failure.error, error], "update and restart failed", {
                    cause: error,
                });
            }
            throw error;
        }
        if (failure !== undefined) {
            throw failure.error;
        }

        return present(installed, "the activated release");
    }

    /** Remove the application and its registration, leaving the device's state. */
    async uninstall(options: { readonly isNative: boolean }): Promise<void> {
        // require an installed executable, and Windows' removal there
        const { isNative } = options;
        if (!this.environment.process.runtime.isStandalone) {
            throw CliError.of(
                "INSTALLATION_MISMATCH",
                "uninstall using the installed Destack executable",
            );
        }
        const { os } = this.environment.host;
        if (os === "win32" && !isNative) {
            throw CliError.of(
                "INSTALLATION_MISMATCH",
                "uninstall Destack through Windows installed applications",
            );
        }

        // take the installation lock and read how Destack was installed
        const installer = this.installer();
        await using _lock = await installer.lock();
        const distribution = this.distribution();
        const registration = await distribution.record();
        if (!isNative && registration?.method === "nsis") {
            throw CliError.of(
                "INSTALLATION_MISMATCH",
                "uninstall Destack through the operating system's installed applications",
            );
        }

        // remove only what the native package registered, reporting nothing registered
        if (isNative) {
            await distribution.requireNative(registration);
            if (registration === undefined) {
                await this.terminal.print("Destack was not registered.");

                return;
            }
        }

        // find the installed distribution and verify the Mac application's identity before stopping anything
        const installed = await installer.current();
        const application =
            os === "darwin"
                ? (registration?.application ??
                  Distribution.bundle(this.environment.process.executable))
                : undefined;
        if (installed === undefined && application === undefined && !isNative) {
            throw CliError.of(
                "INSTALLED_DISTRIBUTION_MISSING",
                "no installed Destack distribution was found",
            );
        }
        if (application !== undefined) {
            await verifyApplication(
                application,
                this.home.stage.applicationIdentifier,
                this.environment,
            );
        }

        // stop the programs, then remove the files
        await this.#quit();
        await this.#remove(
            [installer.directory, ...(application === undefined ? [] : [application])],
            isNative,
        );
        await this.terminal.print(
            `Destack uninstalled. User state retained at ${this.home.directory}.`,
        );
    }

    /** Supervise the installed machine at login for the current OS user, and start it. */
    async supervise(executable: string): Promise<void> {
        // leave explicitly isolated installations independent of user login configuration
        if (this.home.isCustom || (await this.autostart.matches(executable))) {
            return;
        }

        // stop a detached or supervised process before changing its launch configuration
        await this.stop();
        await this.autostart.register(executable);
        if (this.environment.host.os === "win32") {
            const child = this.environment.process.spawn([executable, "self", "start"], {
                stdin: "ignore",
                stdout: "ignore",
                stderr: "pipe",
                signal: this.environment.clock.timeout(START_TIMEOUT_MILLISECONDS),
            });
            const [code, error] = await Promise.all([
                child.exited,
                new Response(child.stderr).text(),
            ]);
            if (code !== 0) {
                throw CliError.of(
                    "MACHINE_START_FAILED",
                    `machine startup failed: ${error.trim()}`,
                );
            }
        } else {
            await this.start(executable, true);
        }
    }

    /** Start the machine from an executable, reusing a running one. */
    async start(executable: string, isStandalone: boolean): Promise<void> {
        await this.machine.start(MACHINE_ENTRY, executable, isStandalone);
    }

    /** Stop the running machine, reporting whether one ran. */
    async stop(): Promise<boolean> {
        // stop a machine that answers, which may stop between the two calls
        if ((await this.version()) === undefined) {
            return false;
        }
        await this.machine.stop().catch((error: unknown) => {
            if (!MachineProcess.isUnavailable(error)) {
                throw error;
            }
        });

        return true;
    }

    /** Download, stage and activate the stage's latest release, and register it. */
    async #installRelease(source: string | undefined): Promise<void> {
        // open the first installation, refusing to replace a native package with an archive
        const distribution = this.distribution();
        await using updater = await distribution.open(true);
        const application = await distribution.application();
        const registration = await distribution.record();
        if (
            application === undefined &&
            registration !== undefined &&
            registration.method !== "archive"
        ) {
            throw CliError.of(
                "INSTALLATION_MISMATCH",
                "uninstall the native package before installing an archive",
            );
        }

        // stage the latest release, copying the downloaded one's files
        const available = await updater.check();
        if (available === undefined) {
            throw UpdateError.of(
                "ALREADY_INSTALLED",
                "the latest Destack release is installed already",
            );
        }
        const download = await available.download(source === undefined ? {} : { source });
        const staged = await updater.stage(download);

        // activate it and register the application or the archive
        const installed = await this.restart(updater, staged, application);
        if (application !== undefined) {
            await this.#registerApplication(application);
        } else {
            await this.#registerArchive(installed);
        }
    }

    /** Register a Mac application at its location, and supervise its machine when installed. */
    async #registerApplication(application: string): Promise<void> {
        const executable = await this.distribution().link(
            { method: "application", application },
            { isShell: !this.home.isCustom },
        );
        if (executable !== undefined) {
            await this.supervise(executable);
        }
    }

    /** Register an activated archive under its installation lock. */
    async #registerArchive(installed: InstalledRelease): Promise<void> {
        // register and link the archive in the home, then supervise its machine, under the lock
        const record = {
            method: "archive" as const,
            application: join(installed.directory, "Destack"),
        };
        const executable = await this.distribution().link(record, {
            isShell: !this.home.isCustom,
        });
        await this.supervise(present(executable, "the archive's machine"));
    }

    /** Start the machine that ran in the selected release, requiring it to run that release. */
    async #resume(
        isRunning: boolean,
        selected: InstalledRelease | undefined,
        application: string | undefined,
    ): Promise<void> {
        // supervise the selected release's machine
        const { os } = this.environment.host;
        const machine = Release.executable("destack-machine", os);
        const executable = selected
            ? join(selected.directory, "bin", machine)
            : MachineProcess.executable(this.environment);
        if (os !== "win32" && (await this.autostart.isRegistered())) {
            const program =
                os === "darwin" && application !== undefined
                    ? Distribution.bundled(application, machine)
                    : executable;
            await this.autostart.register(program);
        }

        // start the machine, requiring the selected release
        if (isRunning) {
            await this.start(
                executable,
                selected !== undefined || this.environment.process.runtime.isStandalone,
            );
            const version = await this.version();
            if (selected && version !== selected.release.version) {
                throw CliError.of(
                    "INSTALLATION_MISMATCH",
                    "restarted machine does not match the installed release",
                );
            }
        }
    }

    /** Wait until the machine answers again running a version. */
    async #waitRestart(version: string): Promise<void> {
        // ask until the deadline on the host's clock
        const { clock } = this.environment;
        const deadline = clock.now() + RESTART_TIMEOUT_MILLISECONDS;
        while (clock.now() < deadline) {
            // ask the machine, which stops answering while it restarts
            if ((await this.version()) === version) {
                return;
            }
            await clock.sleep(POLL_MILLISECONDS);
        }

        // refuse a machine that did not come back in time
        throw CliError.of(
            "RESTART_TIMED_OUT",
            `the machine did not restart into ${version}; read ${this.home.directory}/machine.log`,
        );
    }

    /** Remove OS supervision and stop the machine, and wait for the desktop to quit with it. */
    async #quit(): Promise<void> {
        // remove supervision and stop the machine
        const isDesktop = await this.desktop.isRunning();
        await this.autostart.unregister();
        await this.stop();

        // wait for the desktop, which quits once its machine stops
        if (isDesktop) {
            await this.desktop.untilQuit();
        }
    }

    /** Remove the stage's launcher, the distribution files and the application. */
    async #remove(paths: readonly string[], isNative: boolean): Promise<void> {
        // unregister the launcher from the shell and the Linux desktop
        const { home, environment } = this;
        const { stage } = home;
        const bin = join(home.directory, "bin");
        const launcher = join(bin, isNative ? `${stage.command}.cmd` : stage.command);
        await this.distribution().unlink({ isShell: !home.isCustom });

        // remove the closed files and the registration
        for (const path of [launcher, ...paths]) {
            await environment.files.remove(path, { recursive: true, force: true });
        }
        await environment.files.remove(join(this.home.directory, "distribution.json"), {
            force: true,
        });
    }
}

/** Render a distribution after staging: up to date, or a release ready to restart into. */
function render(row: DistributionRow): string {
    return row.staged === null
        ? `Destack ${row.installed} is up to date.`
        : `Destack ${row.staged} is ready to restart into.`;
}
