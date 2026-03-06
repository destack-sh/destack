# Code Lens

## Function Lenses

### Show reference and test lenses

Code lenses should report reference counts and test actions.

```ds:main.ds
function /*lens*/greet(): void {}

greet();

@test
function test_example(): void {}
```

```lsp code_lens
range=0:9-0:14
title=1 reference
command=destack.showReferences

range=5:9-5:21
title=▶ Run test_example
command=destack.runTest
arg=test_example
```
