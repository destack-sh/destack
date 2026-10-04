import { Release } from "@destack/update/release";
import manifest from "../../../../@destack/desktop/package.json" with { type: "json" };

/** The standalone commands each distribution ships in `bin` and beside its native desktop. */
export const COMMANDS = ["destack", "destack-daemon", "destack-sandbox", "destack-build"];

/** Version injected into every component of this distribution. */
export const version = new Release(
    process.env["DESTACK_RELEASE_VERSION"] ?? manifest.version,
    Release.target(process.platform, process.arch),
).version;

/** Read the Git commit a checkout has checked out. */
export async function readCommit(root: string): Promise<string> {
    // ask Git for the full object name of HEAD
    const child = Bun.spawn(["git", "rev-parse", "--verify", "HEAD"], {
        cwd: root,
        stdout: "pipe",
        stderr: "inherit",
    });
    const [code, output] = await Promise.all([child.exited, new Response(child.stdout).text()]);
    if (code !== 0) {
        throw new Error(`git rev-parse exited with status ${code}`);
    }

    return output.trim();
}
