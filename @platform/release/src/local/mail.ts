import type { Clock } from "@destack/environment";

/** How long a sign-in waits for the universe to print its code: a sign-in mail's send. */
const PRINT_MILLISECONDS = 30_000;

/** How often a sign-in looks for the printed code. */
const POLL_MILLISECONDS = 100;

/** The line of a printed sign-in mail with its code, as `AccountMail.code` writes it. */
const CODE_LINE = /Your Destack sign-in code is (\w+)\./u;

/** The mail a local universe prints, line by line, which sign-ins read their codes from. */
export class PrintedMail {
    /** The printed lines, oldest first. */
    readonly lines: string[] = [];

    /** Keep a printed line. */
    push(line: string): void {
        this.lines.push(line);
    }

    /** Read the sign-in code printed after a line, waiting until the universe prints one. */
    async code(after: number, clock: Pick<Clock, "now" | "sleep">): Promise<string> {
        // read each line printed since, until the code's line or the deadline
        const deadline = clock.now() + PRINT_MILLISECONDS;
        let read = after;
        for (;;) {
            // take the code of a printed line
            const code = this.lines
                .slice(read)
                .map((line) => CODE_LINE.exec(line)?.[1])
                .find((found) => found !== undefined);
            read = this.lines.length;
            if (code !== undefined) {
                return code;
            }
            // refuse once the deadline passed, else look again
            else if (clock.now() >= deadline) {
                throw new Error("the universe printed no sign-in code");
            } else {
                await clock.sleep(POLL_MILLISECONDS);
            }
        }
    }
}
