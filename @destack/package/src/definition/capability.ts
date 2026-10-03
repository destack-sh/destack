import { defineSchema, schema } from "@destack/schema";
import { PackageError } from "../error/index.ts";

/** Why a package needs a capability, shown when a person consents to it. */
const Reason = schema.string().min(1);

/** Declare a capability of the given fields, with its reason and whether installation may decline it. */
function capability<const Fields extends Readonly<Record<string, schema.Schema>>>(fields: Fields) {
    return schema.object({
        ...fields,
        /** Why the package needs it, shown at consent. */
        reason: Reason,
        /** Whether an installation may decline it, the package failing loudly where it uses it ungranted. */
        optional: schema.boolean().exactOptional(),
    });
}

/** A host a workload may connect to: a name or `*.` wildcard with an optional port, or `*` for any host. */
const HostPattern = schema
    .string()
    .regex(/^(?:\*|(?:\*\.)?[a-z0-9-]+(?:\.[a-z0-9-]+)*(?::\d{1,5})?)$(?![\s\S])/u);

/** Access to a directory: reading it, or reading and writing it. */
const Access = schema.enum(["read", "write"]);

/** What a package may reach beyond its sandbox, each consented to at installation. */
const capabilitiesSchema = defineSchema(
    schema.object({
        /** Running the workloads in a full Bun process instead of a workerd isolate, with the `bun` runtime. */
        process: schema.object({ reason: Reason }).exactOptional(),
        /** Outbound connections to the listed hosts. */
        network: capability({ connect: schema.array(HostPattern).min(1) }).exactOptional(),
        /** Listening for connections on loopback. */
        listen: capability({}).exactOptional(),
        /** The directories a person grants the installation, read or written as the package declares. */
        fs: capability({ access: Access }).exactOptional(),
        /** The host environment variables it may read. */
        env: capability({
            names: schema.array(schema.string().regex(/^[A-Za-z_][A-Za-z0-9_]*$/u)).min(1),
        }).exactOptional(),
        /** The commands its processes may spawn. */
        run: capability({
            commands: schema.array(schema.string().regex(/^[A-Za-z0-9._-]+$/u)).min(1),
        }).exactOptional(),
        /** Video from the person's cameras. */
        camera: capability({}).exactOptional(),
        /** Audio from the person's microphones. */
        microphone: capability({}).exactOptional(),
        /** The device's position. */
        geolocation: capability({}).exactOptional(),
        /** Capturing the person's screen, a window or a tab. */
        "display-capture": capability({}).exactOptional(),
        /** Reading the clipboard. */
        "clipboard-read": capability({}).exactOptional(),
        /** Writing the clipboard. */
        "clipboard-write": capability({}).exactOptional(),
        /** Showing an element full screen. */
        fullscreen: capability({}).exactOptional(),
        /** MIDI devices. */
        midi: capability({}).exactOptional(),
        /** USB devices. */
        usb: capability({}).exactOptional(),
        /** Human interface devices. */
        hid: capability({}).exactOptional(),
        /** Serial ports. */
        serial: capability({}).exactOptional(),
        /** Bluetooth devices. */
        bluetooth: capability({}).exactOptional(),
        /** Keeping the screen awake. */
        "screen-wake-lock": capability({}).exactOptional(),
        /** Noticing when the person is idle or locks the screen. */
        "idle-detection": capability({}).exactOptional(),
        /** The fonts installed on the device. */
        "local-fonts": capability({}).exactOptional(),
        /** Placing windows across the person's screens. */
        "window-management": capability({}).exactOptional(),
        /** Virtual and augmented reality sessions. */
        "xr-spatial-tracking": capability({}).exactOptional(),
    }),
);

/** The capabilities, by name. */
export const CapabilityName = defineSchema(capabilitiesSchema.keyof());
/** The name of a capability. */
export type CapabilityName = schema.Infer<typeof CapabilityName>;

/** The capabilities a host's sandbox enforces for its workloads. */
export const HOST_CAPABILITIES = CapabilityName.extract([
    "process",
    "network",
    "listen",
    "fs",
    "env",
    "run",
]).options;

/** The capabilities browsers gate, by the Permissions Policy feature they allow, in the order its headers name them. */
export const BROWSER_CAPABILITIES = CapabilityName.exclude(HOST_CAPABILITIES).options;

/** What a package may reach beyond its sandbox, each with its reason. */
export const Capabilities = Object.assign(capabilitiesSchema, {
    /** Narrow capabilities to those an installation runs with: every required one, and the optional ones it allows. */
    grant(declared: Capabilities, allowed: readonly CapabilityName[]): Capabilities {
        return capabilitiesSchema.parse(
            Object.fromEntries(
                Object.entries(declared).filter(
                    ([name, entry]) =>
                        entry !== undefined &&
                        (!("optional" in entry) ||
                            !entry.optional ||
                            allowed.some((allowance) => allowance === name)),
                ),
            ),
        );
    },
    /** Select the named capabilities a workload or view uses, refusing one the package does not declare. */
    select(declared: Capabilities, names: readonly CapabilityName[]): Capabilities {
        // refuse a name the package does not declare
        const undeclared = names.filter((name) => declared[name] === undefined);
        if (undeclared.length > 0) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `the workload uses undeclared capabilities: ${undeclared.join(", ")}`,
            );
        }

        return capabilitiesSchema.parse(
            Object.fromEntries(names.map((name) => [name, declared[name]])),
        );
    },
});
/** What a package may reach beyond its sandbox. */
export type Capabilities = schema.Infer<typeof capabilitiesSchema>;

/** A directory a person grants an installation's `fs` capability, by absolute path. */
export const DirectoryGrant = defineSchema(
    schema.object({
        /** The directory's absolute path on the host: POSIX, a Windows drive or a UNC share. */
        path: schema.string().regex(/^(?:\/|[A-Za-z]:[\\/]|\\\\)[^\0]*$/u),
        /** Whether the installation reads it, or reads and writes it. */
        access: Access,
    }),
);
/** A directory a person grants an installation's `fs` capability. */
export type DirectoryGrant = schema.Infer<typeof DirectoryGrant>;
