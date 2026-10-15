import { defineSchema, schema } from "@destack/schema";

/** The operating systems, as User-Agent Client Hints name them in `Sec-CH-UA-Platform`. */
export const PLATFORMS = [
    "Android",
    "Chrome OS",
    "Chromium OS",
    "iOS",
    "Linux",
    "macOS",
    "Windows",
    "Unknown",
] as const;

/** The operating systems by Node's platform names. */
const NODE_PLATFORMS: Readonly<Record<string, (typeof PLATFORMS)[number]>> = {
    darwin: "macOS",
    win32: "Windows",
    linux: "Linux",
    android: "Android",
};

/** An operating system, as User-Agent Client Hints name it. */
export const Platform = Object.assign(defineSchema(schema.enum(PLATFORMS)), {
    /** Read the operating system of a Node platform name, absent for one outside the list. */
    ofNode(name: string): (typeof PLATFORMS)[number] | undefined {
        return Object.hasOwn(NODE_PLATFORMS, name) ? NODE_PLATFORMS[name] : undefined;
    },

    /** Read an operating system a browser hints, absent for one outside the list. */
    ofHint(hinted: string): (typeof PLATFORMS)[number] | undefined {
        const parsed = Platform.safeParse(hinted);

        return parsed.success ? parsed.data : undefined;
    },
});
/** An operating system, as User-Agent Client Hints name it. */
export type Platform = schema.Infer<typeof Platform>;
