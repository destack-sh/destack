# Destack (.NET)

.NET client for Destack.
This package is intended to publish to NuGet as `Destack`.

## Usage

```csharp
using Destack;

var client = new Client();
Console.WriteLine(client.Backend);
Console.WriteLine(client.Version());
Console.WriteLine(client.CapiAbiVersion());
Console.WriteLine(client.CapiIsAvailable());
```
