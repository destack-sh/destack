Declare shared resources for applications in a space.

## Usage

Set the copied package name to your account's stack package, such as `@florian/stack`.

```ts
import { credentials, database, files } from "@florian/stack";
```

The `./space` export selects the resources managed from source. Applications can also be installed
through the CLI or UI.

```ts
import { personal } from "@florian/stack/space";
```
