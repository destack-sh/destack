import { schema } from "@destack/schema";
import { PackageError } from "../error/index.ts";

/** A host a workload may connect to: a name or `*.` wildcard with an optional port, or `*`. */
const HostPattern = schema
    .string()
    .regex(/^(?:\*|(?:\*\.)?[a-z0-9-]+(?:\.[a-z0-9-]+)*(?::\d{1,5})?)$/u);

/** Access to a directory: reading it, or reading and writing it. */
const Access = schema.enum(["read", "write"]);

/** The directory accesses from the widest, which a merge takes first. */
const ACCESS_ORDER = ["write", "read"] as const;

/** What a package's workloads and views may use beyond the bare sandbox, by name. */
const sandboxSchema = schema
    .object({
        /** Running the workloads in a full Bun process instead of a workerd isolate. */
        process: schema.object({}).strict().exactOptional(),
        /** Outbound connections to the listed hosts. */
        network: schema
            .object({ connect: schema.array(HostPattern).min(1) })
            .strict()
            .exactOptional(),
        /** Serving connections on loopback. */
        server: schema.object({}).strict().exactOptional(),
        /** The directories a person grants, read or written as the package declares. */
        files: schema.object({ access: Access }).strict().exactOptional(),
        /** The host environment variables it may read. */
        env: schema
            .object({
                names: schema.array(schema.string().regex(/^[A-Za-z_][A-Za-z0-9_]*$/u)).min(1),
            })
            .strict()
            .exactOptional(),
        /** The commands its processes may spawn. */
        run: schema
            .object({
                commands: schema.array(schema.string().regex(/^[A-Za-z0-9._-]+$/u)).min(1),
            })
            .strict()
            .exactOptional(),
        /** Video from the person's cameras. */
        camera: schema.object({}).strict().exactOptional(),
        /** Audio from the person's microphones. */
        microphone: schema.object({}).strict().exactOptional(),
        /** The device's position. */
        geolocation: schema.object({}).strict().exactOptional(),
        /** Capturing the person's screen, a window or a tab. */
        "display-capture": schema.object({}).strict().exactOptional(),
        /** Reading the clipboard. */
        "clipboard-read": schema.object({}).strict().exactOptional(),
        /** Writing the clipboard. */
        "clipboard-write": schema.object({}).strict().exactOptional(),
        /** Showing an element full screen. */
        fullscreen: schema.object({}).strict().exactOptional(),
        /** MIDI devices. */
        midi: schema.object({}).strict().exactOptional(),
        /** USB devices. */
        usb: schema.object({}).strict().exactOptional(),
        /** Human interface devices. */
        hid: schema.object({}).strict().exactOptional(),
        /** Serial ports. */
        serial: schema.object({}).strict().exactOptional(),
        /** Bluetooth devices. */
        bluetooth: schema.object({}).strict().exactOptional(),
        /** Keeping the screen awake. */
        "screen-wake-lock": schema.object({}).strict().exactOptional(),
        /** Noticing when the person is idle or locks the screen. */
        "idle-detection": schema.object({}).strict().exactOptional(),
        /** The fonts installed on the device. */
        "local-fonts": schema.object({}).strict().exactOptional(),
        /** Placing windows across the person's screens. */
        "window-management": schema.object({}).strict().exactOptional(),
        /** Virtual and augmented reality sessions. */
        "xr-spatial-tracking": schema.object({}).strict().exactOptional(),
    })
    .strict();

/** The names of the sandbox's abilities. */
export const SandboxName = sandboxSchema.keyof();
/** The name of one of the sandbox's abilities. */
export type SandboxName = schema.Infer<typeof SandboxName>;

/** The abilities a host's sandbox enforces for its workloads. */
export const HOST_SANDBOX = SandboxName.extract([
    "process",
    "network",
    "server",
    "files",
    "env",
    "run",
]).options;

/** The abilities browsers gate by Permissions Policy feature, in their header order. */
export const BROWSER_SANDBOX = SandboxName.exclude(HOST_SANDBOX).options;

/** What a package's workloads and views may use beyond the bare sandbox. */
export const Sandbox = Object.assign(sandboxSchema, {
    /** Join sandboxes into one: every ability of each, lists united, writing over reading. */
    merge(...sandboxes: readonly Sandbox[]): Sandbox {
        // take every ability, the later entry of each name
        const abilities = Object.fromEntries(
            sandboxes.flatMap((sandbox) => Object.entries(sandbox)),
        );

        // unite the listed hosts, variables and commands
        const connect = unite(sandboxes.map((sandbox) => sandbox.network?.connect));
        const names = unite(sandboxes.map((sandbox) => sandbox.env?.names));
        const commands = unite(sandboxes.map((sandbox) => sandbox.run?.commands));

        // take writing over reading for directories
        const access = new Set(sandboxes.map((sandbox) => sandbox.files?.access));
        const files = ACCESS_ORDER.find((each) => access.has(each));

        return sandboxSchema.parse({
            ...abilities,
            ...(connect.length === 0 ? {} : { network: { connect } }),
            ...(names.length === 0 ? {} : { env: { names } }),
            ...(commands.length === 0 ? {} : { run: { commands } }),
            ...(files === undefined ? {} : { files: { access: files } }),
        });
    },

    /** Write the Permissions Policy allowing a page exactly the browser abilities of a sandbox. */
    policy(sandbox: Sandbox): string {
        return BROWSER_SANDBOX.map(
            (name) => `${name}=${sandbox[name] === undefined ? "()" : "(self)"}`,
        ).join(", ");
    },

    /** Select the abilities a workload or view uses, refusing undeclared ones. */
    select(declared: Sandbox, names: readonly SandboxName[]): Sandbox {
        // refuse a name the package does not declare
        const undeclared = names.filter((name) => declared[name] === undefined);
        if (undeclared.length > 0) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `the workload uses undeclared sandbox abilities: ${undeclared.join(", ")}`,
            );
        }

        return sandboxSchema.parse(Object.fromEntries(names.map((name) => [name, declared[name]])));
    },
});
/** What a package's workloads and views may use beyond the bare sandbox. */
export type Sandbox = schema.Infer<typeof sandboxSchema>;

/** A directory a person grants an installation's `files` ability, by absolute path. */
export const DirectoryGrant = schema.object({
    /** The directory's absolute path on the host: POSIX, a Windows drive or a UNC share. */
    path: schema.string().regex(/^(?:\/|[A-Za-z]:[\\/]|\\\\)[^\0]*$/u),
    /** Whether the installation reads it, or reads and writes it. */
    access: Access,
});
/** A directory a person grants an installation's `files` ability. */
export type DirectoryGrant = schema.Infer<typeof DirectoryGrant>;

/** Unite lists, each value once, in first-seen order. */
function unite(lists: readonly (readonly string[] | undefined)[]): string[] {
    return [...new Set(lists.flatMap((list) => list ?? []))];
}
