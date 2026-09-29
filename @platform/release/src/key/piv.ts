import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

/** Call the installed Yubico library with private input and sanitized diagnostics. */
export function invokePiv<Result>(request: object): Result {
    // use the Python environment maintained by the installed YubiKey Manager
    const executable = Bun.which("ykman");
    if (!executable) {
        throw new Error("install YubiKey Manager before enrollment");
    }
    const interpreter = readFileSync(executable, "utf8").split("\n")[0]!.slice(2).trim();
    if (!interpreter.startsWith("/") || interpreter.includes(" ")) {
        throw new Error(
            "expected a YubiKey Manager installation with an absolute Python interpreter",
        );
    }

    // pass secrets through a private pipe, without forwarding the Bitwarden session
    const environment = { ...process.env };
    delete environment.BW_SESSION;
    delete environment.BW_PASSWORD;
    const result = spawnSync(
        interpreter,
        ["-B", fileURLToPath(new URL("./piv.py", import.meta.url))],
        {
            input: JSON.stringify(request),
            encoding: "utf8",
            timeout: 300_000,
            stdio: ["pipe", "pipe", "pipe"],
            env: environment,
        },
    );
    if (result.error || result.status !== 0) {
        const category = result.stderr.match(/^PIV operation failed: ([A-Za-z]+)$/m)?.[1];
        throw new Error(
            `PIV operation failed${category ? ` (${category})` : ""}; retain the Bitwarden record and inspect the device before retrying`,
        );
    }

    return JSON.parse(result.stdout) as Result;
}
