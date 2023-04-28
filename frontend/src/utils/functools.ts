export function reverseRecord<T extends PropertyKey, U extends PropertyKey>(input: Record<T, U>) {
  return Object.fromEntries(Object.entries(input).map(([key, value]) => [value, key])) as Record<U, T>;
}

export function dedent(input: string, levels: number): string {
  const spacesToRemove = levels * 2;
  const inputLines = input.split("\n");
  const dedentedLines: string[] = [];

  for (const line of inputLines) {
    if (line.startsWith(" ".repeat(spacesToRemove))) {
      dedentedLines.push(line.slice(spacesToRemove));
    } else if (line.startsWith("\t".repeat(levels))) {
      dedentedLines.push(line.slice(levels));
    } else {
      dedentedLines.push(line);
    }
  }

  return dedentedLines.join("\n");
}
