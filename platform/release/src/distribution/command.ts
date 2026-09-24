/** Run a release tool with inherited diagnostics and a bounded lifetime. */
export async function run(
    command: string,
    arguments_: string[],
    cwd = process.cwd(),
    environment: Record<string, string> = {},
): Promise<void> {
    const child = Bun.spawn([command, ...arguments_], {
        cwd,
        env: { ...process.env, ...environment },
        stdin: "ignore",
        stdout: "inherit",
        stderr: "inherit",
        timeout: 20 * 60 * 1000,
    });
    const code = await child.exited;
    if (code !== 0) {
        throw new Error(`${command} exited with status ${code}`);
    }
}
