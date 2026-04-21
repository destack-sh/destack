# Resolution Across Modules

## tests

### union method call sees inherent extensions across modules

> Inherent extensions are visible wherever the type is imported.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }

extension of Cat {
    speak(): string { "meow" }
}

extension of Dog {
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

export extension CatTalk of Cat {
    speak(): string { "meow" }
}

export extension DogTalk of Dog {
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

export extension CatTalk of Cat {
    speak(): string { "meow" }
}

export extension DogTalk of Dog {
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

### union method call resolves through re-exported named extensions

> Re-exported named extensions should remain visible when imported by name.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }
```

```ds:extensions.ds
import { Cat, Dog } from "./types.ds"

export extension CatTalk of Cat {
    speak(): string { "meow" }
}

export extension DogTalk of Dog {
    speak(): string { "woof" }
}
```

```ds:barrel.ds
export { CatTalk, DogTalk } from "./extensions.ds"
```

```ds:main.ds
import { Cat, Dog } from "./types.ds"
import { CatTalk, DogTalk } from "./barrel.ds"

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string;
```

### namespace extension imports do not implicitly enable named extension resolution

> Namespace imports should not implicitly activate named extensions without explicit bindings.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }
```

```ds:extensions.ds
import { Cat, Dog } from "./types.ds"

export extension CatTalk of Cat {
    speak(): string { "meow" }
}

export extension DogTalk of Dog {
    speak(): string { "woof" }
}
```

```ds:main.ds
import { Cat, Dog } from "./types.ds"
import * as ext from "./extensions.ds"

declare function getPet(): Cat | Dog;

getPet().speak();
```

- contains: does not exist

### union method call requires all named extension variants to be visible

> Union method calls require named extension visibility of every union variant.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }
```

```ds:extensions.ds
import { Cat, Dog } from "./types.ds"

export extension CatTalk of Cat {
    speak(): string { "meow" }
}

export extension DogTalk of Dog {
    speak(): string { "woof" }
}
```

```ds:main.ds
import { Cat, Dog } from "./types.ds"
import { CatTalk } from "./extensions.ds"

declare function getPet(): Cat | Dog;

getPet().speak();
```

- contains: does not exist

### union method call resolves through renamed named extension imports

> Renamed named extension imports preserve union method resolution.

```ds:types.ds
export struct Cat { name: string }
export struct Dog { name: string }
```

```ds:extensions.ds
import { Cat, Dog } from "./types.ds"

export extension CatTalk of Cat {
    speak(): string { "meow" }
}

export extension DogTalk of Dog {
    speak(): string { "woof" }
}
```

```ds:main.ds
import { Cat, Dog } from "./types.ds"
import { CatTalk as CatSpeak, DogTalk as DogSpeak } from "./extensions.ds"

declare function getPet(): Cat | Dog;

const sound = getPet().speak();
sound satisfies string;
```
