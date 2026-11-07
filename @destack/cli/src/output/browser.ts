import type { Environment, OperatingSystem } from "@destack/environment";
import { CliError } from "../error/index.ts";

/** The command that opens a URL in the default browser on each operating system. */
const OPENERS: Readonly<Record<OperatingSystem, readonly string[]>> = {
    darwin: ["/usr/bin/open"],
    linux: ["xdg-open"],
    win32: ["rundll32.exe", "url.dll,FileProtocolHandler"],
};

/** Open a URL in the host's default browser. */
export async function browse(
    url: string,
    environment: Environment<"process" | "host">,
): Promise<void> {
    // hand the URL to the operating system's opener
    const opener = OPENERS[environment.host.os];
    const child = environment.process.spawn([...opener, url], {
        stdin: "ignore",
        stdout: "ignore",
        stderr: "inherit",
    });
    const code = await child.exited;
    if (code !== 0) {
        throw CliError.of("BROWSER_LAUNCH_FAILED", `the browser did not open ${url}: ${code}`);
    }
}
