Start a TypeScript package with a shared stack dependency.

```ts
import type { SettingContext } from "@destack/setting";
import { language, appearance } from "./settings/index.ts";

export async function readSettings(settings: SettingContext) {
    const result = await settings.resolve({ language, appearance });

    return {
        language: result.language.value,
        appearance: result.appearance.value,
    };
}
```
