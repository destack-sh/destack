# @destack/sandbox

Run macOS processes with explicit filesystem and network permissions through a separate sandbox manager.

```ts
import { Sandbox } from "@destack/sandbox";

await using sandbox = await Sandbox.start({
    executable: process.execPath,
    arguments: ["run", "--no-env-file", "/application/main.ts"],
    directory: "/application",
    environment: {},
    read: ["/application"],
    write: ["/storage"],
    network: ["api.example.com"],
});

// drain both streams while the workload runs
sandbox.stdout.pipe(process.stdout);
sandbox.stderr.pipe(process.stderr);
const exit = await sandbox.exited;
console.log(exit);
```
