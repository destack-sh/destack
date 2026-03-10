# Destack (Swift)

Swift client for Destack.
This package is intended to publish with Swift Package Manager as `Destack`.

## Usage

```swift
import Destack

let client = Client()
print(client.backend)
print(client.version())
```

## Testing

Run these from the repository root.

```sh
# focused local loop
just bridge/test

# clean gate
just bridge/quick

# exhaustive gate
just bridge/full
```
