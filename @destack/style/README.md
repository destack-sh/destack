# @destack/style

Define styles with StyleX.

## Styles

`create` declares named styles, and the build compiles each property and value to one CSS class.

```ts
import * as style from "@destack/style";

const styles = style.create({ container: { display: "flex" } });
```
