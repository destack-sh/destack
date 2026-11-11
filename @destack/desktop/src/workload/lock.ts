import { join } from "node:path";
import type { Environment, FileLock } from "@destack/environment";
import { DesktopError } from "../error/index.ts";

/** How long a desktop may take to quit once its machine stops. */
const QUIT_TIMEOUT_MILLISECONDS = 10_000;

/** The name of the lock file of a running desktop. */
const LOCK_FILE = "desktop.lock";

/** The lock a running desktop has in a machine's directory, which a second desktop is refused. */
export class DesktopLock {
    /** The lock file. */
    readonly path: string;
    /** The files the lock lives in, and the clock a wait for it reads. */
    readonly #environment: Environment<"files">;

    /** Find the lock of a machine's directory. */
    constructor(home: string, environment: Environment<"files">) {
        this.path = join(home, LOCK_FILE);
        this.#environment = environment;
    }

    /** Take the lock while this desktop runs, refusing a second desktop of the same user. */
    async acquire(): Promise<FileLock> {
        // refuse the lock another desktop has
        const lock = await this.#environment.files.tryLock(this.path, "exclusive");
        if (lock === undefined) {
            throw DesktopError.of(
                "DESKTOP_BLOCKED",
                "cannot start the desktop: another Destack desktop runs for this user",
            );
        }

        return lock;
    }

    /** Report whether a desktop runs for this user. */
    async isHeld(): Promise<boolean> {
        // NOTE #Robustness: the probe takes the lock for an instant, refusing a desktop starting meanwhile
        await using lock = await this.#environment.files.tryLock(this.path, "exclusive");

        return lock === undefined;
    }

    /** Wait until the running desktop quits, as it does once its machine stops. */
    async released(): Promise<void> {
        // take the lock once released, and let it go at once
        const { files, clock } = this.#environment;
        await using released = await files.lock(this.path, "exclusive", {
            signal: clock.timeout(QUIT_TIMEOUT_MILLISECONDS),
        });
    }
}
