# destack (Dart)

Dart client for Destack.
This package is intended to publish to pub.dev as `destack`.

## Usage

```dart
import "package:destack/destack.dart";

void main() {
  final client = DestackClient();
  print(client.backend);
  print(client.version());
  print(client.capiAbiVersion());
  print(client.capiIsAvailable());
}
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
