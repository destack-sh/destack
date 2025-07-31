interface Flag {
  type: "string" | "number" | "boolean";
  shortFlag?: string;
  default?: any;
  description?: string;
}

interface Flags {
  [key: string]: Flag;
}

interface ParsedArgs {
  input: string[];
  flags: Record<string, any>;
  help: string;
  showHelp: () => void;
}

interface ParserOptions {
  description?: string;
  usage?: string;
  examples?: string[];
  flags?: Flags;
}

/** Parse command line arguments */
export function parseArgs(options: ParserOptions = {}): ParsedArgs {
  const args = process.argv.slice(2);
  const input: string[] = [];
  const flags: Record<string, any> = {};
  
  // initialize defaults
  if (options.flags) {
    for (const [name, flag] of Object.entries(options.flags)) {
      if (flag.default !== undefined) {
        flags[name] = flag.default;
      }
    }
  }

  // parse arguments
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    
    if (arg.startsWith("--") || arg.startsWith("-")) {
      const isLong = arg.startsWith("--");
      const flagName = isLong ? arg.slice(2) : arg.slice(1);
      
      // find the flag definition
      let flagDef: Flag | undefined;
      let actualName: string = flagName;
      
      if (options.flags) {
        if (isLong) {
          flagDef = options.flags[flagName];
        } else {
          // short flag - find the corresponding long flag
          for (const [name, flag] of Object.entries(options.flags)) {
            if (flag.shortFlag === flagName) {
              actualName = name;
              flagDef = flag;
              break;
            }
          }
        }
      }
      
      if (flagDef) {
        if (flagDef.type === "boolean") {
          flags[actualName] = true;
        } else {
          const nextArg = args[i + 1];
          if (nextArg && !nextArg.startsWith("-")) {
            if (flagDef.type === "number") {
              flags[actualName] = parseInt(nextArg, 10);
            } else {
              flags[actualName] = nextArg;
            }
            i++;
          }
        }
      }
    } else {
      input.push(arg);
    }
  }
  
  const help = generateHelp(options);
  
  return {
    input,
    flags,
    help,
    showHelp: () => {
      console.log(help);
      process.exit(0);
    },
  };
}

function generateHelp(options: ParserOptions): string {
  const lines: string[] = [];
  
  if (options.description) {
    lines.push(options.description);
    lines.push("");
  }
  
  if (options.usage) {
    lines.push("Usage");
    lines.push(`  ${options.usage}`);
    lines.push("");
  }
  
  if (options.flags && Object.keys(options.flags).length > 0) {
    lines.push("Options");
    for (const [name, flag] of Object.entries(options.flags)) {
      let line = "  ";
      if (flag.shortFlag) {
        line += `-${flag.shortFlag}, `;
      }
      line += `--${name}`;
      if (flag.description) {
        line += `    ${flag.description}`;
      }
      if (flag.default !== undefined) {
        line += ` (default: ${flag.default})`;
      }
      lines.push(line);
    }
    lines.push("");
  }
  
  if (options.examples && options.examples.length > 0) {
    lines.push("Examples");
    for (const example of options.examples) {
      lines.push(`  ${example}`);
    }
  }
  
  return lines.join("\n");
}