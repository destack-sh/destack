# Resolution Across Modules

## tests

### union method call sees inherent extensions across modules

> Inherent extensions are visible wherever the type is imported.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }

extension for Cat {
    speak(): string { "meow" }
}

extension for Dog {
    speak(): string { "woof" }
}
```

```ds:main.ds
import { Cat, Dog } from "./types.ds"

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string;
```

### union method call requires named extension imports

> Named extensions on foreign types must be imported to participate in resolution.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }
```

```ds:extensions.ds
import { Cat, Dog } from "./types.ds"

export extension CatTalk for Cat {
    speak(): string { "meow" }
}

export extension DogTalk for Dog {
    speak(): string { "woof" }
}
```

```ds:main.ds
import { Cat, Dog } from "./types.ds"

declare function getPet(): Cat | Dog;

getPet().speak();
```

- contains: does not exist

### union method call resolves with named extension imports

> Named extensions are visible when explicitly imported.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }
```

```ds:extensions.ds
import { Cat, Dog } from "./types.ds"

export extension CatTalk for Cat {
    speak(): string { "meow" }
}

export extension DogTalk for Dog {
    speak(): string { "woof" }
}
```

```ds:main.ds
import { Cat, Dog } from "./types.ds"
import { CatTalk, DogTalk } from "./extensions.ds"

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string;
```
