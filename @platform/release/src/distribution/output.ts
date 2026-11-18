import type { Environment } from "@destack/environment";

/** The width of the status lines' name column: the longest service, `eu-central-tail`. */
const NAME_WIDTH = 15;

/** Write a line of a release command's output to standard output. */
export function print(text: string, environment: Environment<"process">): void {
    environment.process.stdout.writeSync(`${text}\n`);
}

/** Write a status line under a name to standard output. */
export function printStatus(name: string, line: string, environment: Environment<"process">): void {
    environment.process.stdout.writeSync(`${name.padEnd(NAME_WIDTH)}| ${line}\n`);
}
