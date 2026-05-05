# Extension Unions

Union member calls require a visible method for every union member.

## inherent

### inherent extensions resolve on unions

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

## named

### named extensions require imports on unions

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

### named extension imports resolve unions

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

### re-exported named extensions resolve unions

> Re-exported named extensions remain visible when imported by name.

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

### namespace imports do not activate named extensions

> Namespace imports do not implicitly activate named extensions.

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

### union calls require every extension

> Union method calls require visible extensions for every union member.

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

### renamed named extensions resolve unions

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
