# Type Hierarchy

## Subtypes

### interface subtypes

Type hierarchy should list the subtypes of the selected interface.

```ds:main.ds
interface /*type_hierarchy*/Animal {}

class Dog implements Animal {}
```

```lsp type_hierarchy_subtypes
item=main.ds:3:1-3:31
name=Dog
kind=class
selection=main.ds:3:7-3:10
```

### Grow and prune interface subtypes across four states
Type hierarchy should reflect each intermediate subtype set as implementations appear and disappear.

```ds:main.ds
interface /*type_hierarchy*/Animal {}

class Dog implements Animal {}
```

```ds:main.ds[1]
interface /*type_hierarchy*/Animal {}

class Dog implements Animal {}
class Cat implements Animal {}
```

```ds:main.ds[2]
interface /*type_hierarchy*/Animal {}

class Cat implements Animal {}
class Bird implements Animal {}
```

```ds:main.ds[3]
interface /*type_hierarchy*/Animal {}

class Bird implements Animal {}
```

```lsp type_hierarchy_subtypes type_hierarchy [0]
item=main.ds:3:1-3:31
name=Dog
kind=class
selection=main.ds:3:7-3:10
```

```lsp type_hierarchy_subtypes type_hierarchy [1]
item=main.ds:3:1-3:31
name=Dog
kind=class
selection=main.ds:3:7-3:10

item=main.ds:4:1-4:31
name=Cat
kind=class
selection=main.ds:4:7-4:10
```

```lsp type_hierarchy_subtypes type_hierarchy [2]
item=main.ds:3:1-3:31
name=Cat
kind=class
selection=main.ds:3:7-3:10

item=main.ds:4:1-4:32
name=Bird
kind=class
selection=main.ds:4:7-4:11
```

```lsp type_hierarchy_subtypes type_hierarchy [3]
item=main.ds:3:1-3:32
name=Bird
kind=class
selection=main.ds:3:7-3:11
```

## Supertypes

### class supertypes

Type hierarchy should list the supertypes of the selected class.

```ds:main.ds
class Base {}

class /*type_hierarchy*/Derived extends Base {}
```

```lsp type_hierarchy_supertypes
item=main.ds:1:1-1:14
name=Base
kind=class
selection=main.ds:1:7-1:11
```
