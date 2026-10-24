/** The operating systems a process runs on, as Node's `process.platform` has them. */
export type OperatingSystem = "darwin" | "linux" | "win32";

/** The processor architectures a process runs on, as Node's `process.arch` has them. */
export type Architecture = "arm64" | "x64";

/** The machine a process runs on. */
export interface Host {
    /** The operating system. */
    readonly os: OperatingSystem;
    /** The processor architecture. */
    readonly architecture: Architecture;
    /** The operating system's release, as `uname -r` prints it. */
    readonly version: string;
    /** The machine's host name. */
    readonly hostname: string;
    /** The person's home directory. */
    readonly home: string;
}
