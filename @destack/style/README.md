# @destack/style

Define styles with StyleX.

## Styles

`create` declares named styles, and the build compiles each property and value to one CSS class.

```ts
import * as style from "@destack/style";

const styles = style.create({ container: { display: "flex" } });
```

## Builds

`styleExtension` compiles StyleX styles into the output's stylesheet in builds of every package whose dependency closure includes `@destack/style`.

```ts
import { Button } from "@destack/ui/button"; // the build compiles the button's styles through @destack/ui's dependency on @destack/style
```
