/** Write a line of a release command's output to standard output. */
export function print(text: string): void {
    process.stdout.write(`${text}\n`);
}
