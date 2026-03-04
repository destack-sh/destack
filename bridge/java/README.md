# Destack (Java)

Java client for Destack.
This package is intended to publish to Maven Central as `industries.symbol.destack:destack-java`.

## Usage

```java
import industries.symbol.destack.Client;

Client client = new Client();
System.out.println(client.backend());
System.out.println(client.version());
System.out.println(client.capiAbiVersion());
System.out.println(client.capiIsAvailable());
```
