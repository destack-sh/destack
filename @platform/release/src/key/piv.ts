import { fileURLToPath } from "node:url";
import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { communicate } from "../distribution/command.ts";

/** How long one PIV operation may run, in milliseconds: a touch and a PIN entry. */
const PIV_TIMEOUT_MILLISECONDS = 300_000;

/** The variables of the unlocked vault, which the PIV tool never receives. */
const VAULT_VARIABLES: ReadonlySet<string> = new Set(["BW_SESSION", "BW_PASSWORD"]);

/** Call the installed Yubico library with private input and sanitized diagnostics. */
export async function invokePiv<Result>(
    request: object,
    response: schema.Schema<Result>,
    environment: Environment<"files" | "process">,
): Promise<Result> {
    // use the Python environment maintained by the installed YubiKey Manager
    const executable = environment.process.which("ykman");
    if (executable === undefined) {
        throw new Error("install YubiKey Manager before enrollment");
    }
    const [shebang = ""] = (await environment.files.readTextFile(executable)).split("\n");
    const interpreter = shebang.slice(2).trim();
    if (!interpreter.startsWith("/") || interpreter.includes(" ")) {
        throw new Error(
            "expected a YubiKey Manager installation with an absolute Python interpreter",
        );
    }

    // pass secrets through a private pipe, without forwarding the Bitwarden session
    const variables = Object.fromEntries(
        Object.entries(environment.process.variables).filter(
            ([name]) => !VAULT_VARIABLES.has(name),
        ),
    );
    const result = await communicate(
        interpreter,
        ["-B", fileURLToPath(new URL("./piv.py", import.meta.url))],
        environment,
        { input: JSON.stringify(request), variables, timeout: PIV_TIMEOUT_MILLISECONDS },
    );
    const decoder = new TextDecoder();
    if (result.code !== 0) {
        const diagnostics = decoder.decode(result.stderr);
        const category = /^PIV operation failed: ([A-Za-z]+)$/mu.exec(diagnostics)?.[1];
        throw new Error(
            `PIV operation failed${category === undefined ? "" : ` (${category})`}; retain the Bitwarden record and inspect the device before retrying`,
        );
    }

    return response.parse(JSON.parse(decoder.decode(result.stdout)));
}
