# Code Lens

## Function Lenses

### Reference and test lenses

Code lenses should include reference counts and test actions.

```ds
$0function greet(): void {}

greet();

function test_example(): void {}
```

```query code_lens $0
main.ds:1:10-1:15 kind=references title=1 reference
main.ds:5:10-5:22 kind=run_test title=▶ Run test_example
```
