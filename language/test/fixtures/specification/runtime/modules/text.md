# Text

Text modules import as strings.

## imports

### text files import as strings

> Text file extensions use the text loader by default.

```text:readme.txt
Hello, World!
```

```ds:main.ds
import content from "./readme.txt";

content satisfies string;
```

### text import attributes override data defaults

> The text loader can be selected explicitly with an import attribute.

```json:data.json
{ "key": "value" }
```

```ds:main.ds
import content from "./data.json" with { type: "text" };

content satisfies string;
```
