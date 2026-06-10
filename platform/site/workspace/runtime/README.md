# runtime/

Destack's runtime and toolchain are part of the language design rather than an ecosystem to assemble.
Modules carry metadata and conditions, data files import as typed literals, and host access is granted per module instead of ambiently.

Verification works the same way: test, bench, fuzz, and simulation are toolchain verbs, not packages to wire up.

```sh
destack test       # typed tests, no runner to configure
destack bench      # measured work and resource units
destack fuzz       # generators and shrinking
destack simulate   # deterministic fault injection, replayable by seed
```

Host capabilities are deny-by-default, and grants live in source where review happens.
A module states what it may touch, and everything else fails at the binding boundary:

```ds
@policy({
    allow: ["fs.read:./public", "net.connect:https://api.destack.sh"],
})
module {}
```

The files below cover the module system first and the verification tools after.

| file | shows |
| --- | --- |
| [`meta.ds`](meta.ds) | `import.meta` profile facts, baked per target |
| [`conditions.ds`](conditions.ds) | mode/feature gating in code and through file name aliases |
| [`data.ds`](data.ds) | data files imported as exact readonly literals |
| [`tests.ds`](tests.ds) | typed tests in the same toolchain (`destack test`) |
| [`bench.ds`](bench.ds) | benchmark cases measuring work and resource units (`destack bench`) |
| [`simulation.ds`](simulation.ds) | deterministic fault injection against a controlled world |
| [`policy.ds`](policy.ds) | explicit host capability grants per module |
| [`config.json`](config.json) | the data module imported by `data.ds` |
