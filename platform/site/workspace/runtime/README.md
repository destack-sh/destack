# runtime/

Destack ships with a fully integrated, native language runtime that extends and generalises familiar paradigms (like module metadata, conditional inclusion, and typed data imports) into one coherent system.
Oh, and we also have built-in verification tooling and granular deny-by-default permissioning.

The verification tools are plain toolchain verbs:

```sh
destack test       # typed tests
destack bench      # measured work and resource units
destack fuzz       # generators and shrinking
destack simulate   # deterministic fault injection, replayable by seed
```

All of them run ordinary Destack code through the same compiler pipeline, so assertions and reports work on typed values.

Permissioning is deny-by-default: a module declares the host capabilities it may use, the grant lives in source, and any other host access fails at the binding boundary:

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
