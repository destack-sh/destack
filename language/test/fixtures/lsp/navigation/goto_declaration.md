# Goto Declaration

## Local Bindings

### Declaration for a local value

Goto declaration should jump from a local use site to its binding declaration.

```ds:main.ds
const /*declare_def*/value = 1;
const output = /*declare_use*/value;
```

