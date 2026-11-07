import { join } from "node:path";
import { Simulation } from "@destack/environment/simulation";
import { Autostart, StageHome } from "@destack/machine/computer";
import { Stage } from "@destack/update/release";
import { expect, test, refusal } from "@destack/test";
import { type InstalledRelease, Release, type StagedRelease, UpdateError } from "@destack/update";
import { Application } from "@destack/desktop/workload";
import { Setup } from "./setup.ts";

/** The release the machine runs before the update. */
const PREVIOUS = installed("2026.9.1");

/** The staged release the update activates. */
const STAGED: StagedRelease = { ...installed("2026.9.2"), previous: "2026.9.1" };

/** A release installed in its version's directory. */
function installed(version: string): InstalledRelease {
    return {
        release: new Release(version, "aarch64-apple-darwin"),
        sha256: "0".repeat(64),
        directory: `/releases/${version}`,
    };
}

/** The home of the device under test. */
const HOME = "/home/ada/.destack";

/** The Mac the device runs on. */
const environment = new Simulation({
    seed: 1,
    start: 0,
    host: {
        os: "darwin",
        architecture: "arm64",
        version: "24.1.0",
        hostname: "mac",
        home: "/home/ada",
    },
}).environment("mac");

/** The machine executable of a release. */
function machineIn(release: InstalledRelease): string {
    return join(release.directory, "bin", Release.executable("destack-machine", "darwin"));
}

/** A supervision that runs, recording what it does. */
class RunningAutostart extends Autostart {
    /** What the restart did, in order. */
    readonly done: string[];

    /** Supervise a home, recording into a list. */
    constructor(done: string[]) {
        super(HOME, "app.destack.machine.test", environment);
        this.done = done;
    }

    /** Report the machine supervised. */
    override async isRegistered(): Promise<boolean> {
        return true;
    }

    /** Supervise the machine from an executable. */
    override async register(executable: string): Promise<void> {
        this.done.push(`supervise ${executable}`);
    }

    /** Pause the supervision. */
    override async stop(): Promise<void> {
        this.done.push("pause");
    }
}

/** A desktop that runs, recording what it does. */
class RunningDesktop extends Application {
    /** What the restart did, in order. */
    readonly done: string[];

    /** Run a desktop of a home, recording into a list. */
    constructor(done: string[]) {
        super(HOME, environment);
        this.done = done;
    }

    /** Report the desktop running. */
    override async isRunning(): Promise<boolean> {
        return true;
    }

    /** Wait for the desktop to quit. */
    override async untilQuit(): Promise<void> {
        this.done.push("desktop quit");
    }

    /** Launch the desktop application. */
    override async launch(application: string): Promise<void> {
        this.done.push(`launch ${application}`);
    }
}

/** A setup whose machine, supervision and desktop run. */
class RunningSetup extends Setup {
    /** What the restart did, in order. */
    readonly done: string[];
    /** The version the machine runs, absent while stopped. */
    running: string | undefined;
    /** The version the machine runs once started. */
    readonly started: string;

    /** Run a machine of a version, starting in another. */
    constructor(running: string, started: string) {
        const done: string[] = [];
        super({
            home: new StageHome(new Stage("production"), "/home/ada", HOME),
            autostart: new RunningAutostart(done),
            desktop: new RunningDesktop(done),
            environment,
        });
        this.done = done;
        this.running = running;
        this.started = started;
    }

    /** Read the version the machine runs. */
    override async version(): Promise<string | undefined> {
        return this.running;
    }

    /** Start the machine from an executable, running the version it starts in. */
    override async start(executable: string): Promise<void> {
        this.done.push(`start ${executable}`);
        this.running = this.started;
    }

    /** Stop the machine, reporting whether it ran. */
    override async stop(): Promise<boolean> {
        this.done.push("stop");
        const isRunning = this.running !== undefined;
        this.running = undefined;

        return isRunning;
    }
}

/** An updater whose activation ends in a release or fails, recording it beside the device. */
function updater(device: RunningSetup, failure: Error | undefined) {
    let current = PREVIOUS;

    return {
        current: async () => current,
        activate: async (staged: StagedRelease) => {
            device.done.push(`activate ${staged.release.version}`);
            if (failure !== undefined) {
                throw failure;
            }
            current = staged;

            return staged;
        },
    };
}

/** The supervision of an executable, which the Mac's launchd agent runs. */
function supervising(executable: string): string[] {
    return [`supervise ${executable}`];
}

test("stop the machine and the desktop, activate the staged release, and start both again in it", async () => {
    const device = new RunningSetup("2026.9.1", "2026.9.2");

    // restart into the staged release
    const activated = await device.restart(updater(device, undefined), STAGED, undefined);
    expect({ activated, done: device.done }).toEqual({
        activated: STAGED,
        done: [
            "pause",
            "stop",
            "desktop quit",
            "activate 2026.9.2",
            ...supervising(machineIn(STAGED)),
            `start ${machineIn(STAGED)}`,
            `launch ${device.desktop.locateIn(STAGED.directory)}`,
        ],
    });
});

test("start the device again in the previous release when activation fails, and report the failure", async () => {
    const device = new RunningSetup("2026.9.1", "2026.9.1");
    const failure = new Error("the disk is full");

    // fail the activation, and restart into the previous release
    await expect(device.restart(updater(device, failure), STAGED, undefined)).rejects.toBe(failure);
    expect(device.done).toEqual([
        "pause",
        "stop",
        "desktop quit",
        "activate 2026.9.2",
        ...supervising(machineIn(PREVIOUS)),
        `start ${machineIn(PREVIOUS)}`,
        `launch ${device.desktop.locateIn(PREVIOUS.directory)}`,
    ]);
});

test("stay stopped when the activation leaves the files disagreeing with their record", async () => {
    const device = new RunningSetup("2026.9.1", "2026.9.1");
    const failure = UpdateError.of("ACTIVATION_FAILED", "the release record is torn");

    // fail the activation midway
    await expect(device.restart(updater(device, failure), STAGED, undefined)).rejects.toBe(failure);
    expect(device.done).toEqual(["pause", "stop", "desktop quit", "activate 2026.9.2"]);
});

test("refuse a restarted machine running another release than the activated one", async () => {
    const device = new RunningSetup("2026.9.1", "2026.9.1");

    // restart into a machine still reporting the previous release
    expect(await refusal(device.restart(updater(device, undefined), STAGED, undefined))).toEqual([
        "PRECONDITION_FAILED",
        "INSTALLATION_MISMATCH",
        "restarted machine does not match the installed release",
    ]);
});
