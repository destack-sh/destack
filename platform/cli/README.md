# cli

Destack command-line interface.
The `destack` binary for compiling, formatting, and running Destack projects.

## Layout

```
src/
├── main.rs      Entry point
├── cli.rs       CLI argument parsing
├── command/     Subcommands (compile, parse, lex, resolve, etc.)
└── console/     Terminal output utilities
```

