# destack (Go)

Go client for Destack.
This module is intended to publish at `go.destack.sh/destack`.

## Installation

```sh
go get go.destack.sh/destack@latest
```

## API

```go
package main

import (
	"fmt"

	"go.destack.sh/destack"
)

func main() {
	client := destack.NewClient()
	fmt.Println(client.BackendName())
	fmt.Println(client.VersionString())
}
```
