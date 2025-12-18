# Rename

## Local Variables

### Rename local variable

Rename should update all occurrences of a symbol to the new name.

```ds
const foo = 1;
//    ^^^ target

const bar = foo + foo;
console.log(foo);
```

`foo` appears 4 times: its definition and 3 uses. Renaming to `baz` should update all 4 occurrences.

```query rename target "baz"
```

```expected:main
const baz = 1;

const bar = baz + baz;
console.log(baz);
```

## Classes

### Rename class with type references

Renaming a class should update its definition and all type references.

```ds
class Meetup {
//    ^^^^^^ target
    name: string
}

class MeetupLanguage {
    code: string
}

const m: Meetup = new Meetup();
//       ^^^^^^ ref1
//                   ^^^^^^ ref2
```

Renaming `Meetup` to `Event` should only update `Meetup`, not `MeetupLanguage`.

```query rename target "Event"
```

```expected:main
class Event {
    name: string
}

class MeetupLanguage {
    code: string
}

const m: Event = new Event();
```

### Rename class with field type references

Renaming a class used as a field type should work correctly.

```ds
class Meetup {
//    ^^^^^^ target
    name: string
}

enum MeetupLanguage {
    English,
    German,
}

class Registration {
    /// The meetup to register for.
    meetup: Meetup,
//          ^^^^^^ ref
    /// The language preference.
    language: MeetupLanguage,
}
```

Renaming `Meetup` to `Event` should only update `Meetup`, not affect `MeetupLanguage`.

```query rename target "Event"
```

```expected:main
class Event {
    name: string
}

enum MeetupLanguage {
    English,
    German,
}

class Registration {
    /// The meetup to register for.
    meetup: Event,
    /// The language preference.
    language: MeetupLanguage,
}
```
