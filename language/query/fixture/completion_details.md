# Completion Details

## Declaration Documentation

### Resolve callable documentation

The selected completion returns its full declaration and authored documentation.

```tspp main.tspp
/// Format one name.
/// @param name - The name to format.
function formatName(name: string): string { return name; }

const formatted = formatN;
                  ^^^^^^^ prefix
```

```query completion_details main.tspp#prefix@end entry=formatName
@completion_details.item declaration="function formatName(name: string): string" documentation="Format one name.\n\n## Parameters\n\n- `name`: The name to format."
```

## Auto Imports

### Resolve an auto-import edit

The selected completion returns its exact import edit from the original source.

```tspp library.tspp
export function greetFixture(): void {}
```

```tspp main.tspp

^ insertion
function main(): void {
    greetFix;
    ^^^^^^^^ prefix
}
```

```query completion_details main.tspp#prefix@end entry=greetFixture include_auto_imports=true
@completion_details.item declaration="export function greetFixture(): void"
@completion_details.additional_edit range=main.tspp#insertion text="import { greetFixture } from \"./library\";\n"
```
