# Binary

Binary modules import as byte arrays.

## imports

### binary files import as byte arrays

Binary file extensions use the binary loader by default.

```text:icon.png
fake image bytes
```

```ds:main.ds
import icon from "./icon.png.ds";

icon satisfies uint8[];
```

### binary import attributes override text defaults

The binary loader can be selected explicitly with an import attribute.

```text:file.txt
hello
```

```ds:main.ds
import bytes from "./file.txt.ds" with { type: "binary" };

bytes satisfies uint8[];
```
