Start a TypeScript package with a shared stack dependency.

```ts
import { SettingSelection } from "@destack/setting";
import { setting } from "@destack/setting/object";
import { language } from "./settings/index.ts";
import { readLanguage } from "./index.ts";

const selection = SettingSelection.parse({ scope: userId, space: spaceId });
const query = client.subscribe(setting, { where: language.condition(selection) });
const content = readLanguage(selection, await query.read());
```
