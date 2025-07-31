// ANSI color codes
const colors = {
  reset: "\x1b[0m",
  bold: "\x1b[1m",
  dim: "\x1b[2m",
  
  // foreground colors
  black: "\x1b[30m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  magenta: "\x1b[35m",
  cyan: "\x1b[36m",
  white: "\x1b[37m",
  gray: "\x1b[90m",
  
  // bright foreground colors
  brightRed: "\x1b[91m",
  brightGreen: "\x1b[92m",
  brightYellow: "\x1b[93m",
  brightBlue: "\x1b[94m",
  brightMagenta: "\x1b[95m",
  brightCyan: "\x1b[96m",
  brightWhite: "\x1b[97m",
} as const;

type Color = keyof typeof colors;

/** Apply color to text */
export function color(text: string, ...styles: Color[]): string {
  const codes = styles.map(s => colors[s]).join("");
  return `${codes}${text}${colors.reset}`;
}

/** Print colored text to stdout */
export function print(text: string, ...styles: Color[]): void {
  console.log(color(text, ...styles));
}

/** Print text without newline */
export function write(text: string, ...styles: Color[]): void {
  process.stdout.write(color(text, ...styles));
}

/** Print error to stderr */
export function error(text: string): void {
  console.error(color(text, "red"));
}

/** Print warning */
export function warn(text: string): void {
  console.warn(color(text, "yellow"));
}

/** Print success message */
export function success(text: string): void {
  console.log(color(text, "green"));
}

/** Print info message */
export function info(text: string): void {
  console.log(color(text, "cyan"));
}

/** Clear the terminal */
export function clear(): void {
  process.stdout.write("\x1b[2J\x1b[0f");
}

/** Move cursor up n lines */
export function moveUp(lines: number = 1): void {
  process.stdout.write(`\x1b[${lines}A`);
}

/** Clear the current line */
export function clearLine(): void {
  process.stdout.write("\x1b[2K\r");
}

/** Create a simple box around content */
export function box(content: string, padding: number = 1): string {
  const lines = content.split("\n");
  const maxLength = Math.max(...lines.map(l => l.length));
  const width = maxLength + padding * 2;
  
  const result: string[] = [];
  
  // top border
  result.push("┌" + "─".repeat(width) + "┐");
  
  // content with padding
  for (const line of lines) {
    const paddedLine = line.padEnd(maxLength);
    result.push("│" + " ".repeat(padding) + paddedLine + " ".repeat(padding) + "│");
  }
  
  // bottom border
  result.push("└" + "─".repeat(width) + "┘");
  
  return result.join("\n");
}

/** Create a section header */
export function header(text: string, char: string = "="): string {
  const line = char.repeat(text.length);
  return `${line}\n${text}\n${line}`;
}