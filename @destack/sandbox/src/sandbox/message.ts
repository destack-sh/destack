import { schema } from "@destack/schema";

/** Explicit process inputs and host-approved access. */
export const SandboxOptions = schema.object({
    /** Absolute executable path. */
    executable: schema.string().min(1),
    /** Arguments passed unchanged to the executable. */
    arguments: schema.array(schema.string()),
    /** Absolute working directory. */
    directory: schema.string().min(1),
    /** Environment supplied to the workload, excluding inherited host variables. */
    environment: schema.record(schema.string(), schema.string()),
    /** Absolute readable paths, in addition to the executable and required OS runtime files. */
    read: schema.array(schema.string()),
    /** Absolute writable files and directories, also readable. */
    write: schema.array(schema.string()),
    /** Allowed outgoing domains through the supplied proxies, optionally qualified by port. */
    network: schema.array(schema.string()),
    /** Unix sockets available for host-mediated service connections, none when empty. */
    sockets: schema.array(schema.string()),
    /** Where the workload may listen: nowhere, or on loopback ports. */
    listening: schema.enum(["denied", "loopback"]),
});
/** Explicit process inputs and host-approved access. */
export type SandboxOptions = schema.Infer<typeof SandboxOptions>;

/** Process termination reported after sandbox cleanup. */
export const SandboxExit = schema.object({
    /** Exit code, absent when a signal terminates the process. */
    code: schema.int().nullable(),
    /** Terminating signal, absent after a normal exit. */
    signal: schema.string().min(1).nullable(),
});
/** Process termination reported after sandbox cleanup. */
export type SandboxExit = schema.Infer<typeof SandboxExit>;

/** Parent commands accepted by one launcher. */
export const LauncherRequest = schema.discriminatedUnion("type", [
    schema.object({
        /** Start with fixed permissions. */
        type: schema.literal("start"),
        /** Authorized launch inputs. */
        options: SandboxOptions,
    }),
    schema.object({
        /** Stop this workload. */
        type: schema.literal("stop"),
        /** Graceful shutdown duration in milliseconds. */
        gracePeriod: schema.int().min(0),
    }),
]);

/** Launcher lifecycle messages over its private parent connection. */
export const LauncherMessage = schema.discriminatedUnion("type", [
    schema.object({
        /** The command that applies OS restrictions has spawned. */
        type: schema.literal("ready"),
    }),
    schema.object({
        /** The workload has terminated. */
        type: schema.literal("exit"),
        /** Process termination. */
        exit: SandboxExit,
    }),
    schema.object({
        /** Startup or cleanup failed. */
        type: schema.literal("error"),
        /** Failure description. */
        message: schema.string(),
    }),
]);
