# destack (Elixir)

Elixir client for Destack.
This package is intended to publish to Hex as `destack`.
The client uses an Erlang NIF to call the shared `destack_capi` library when available.

## Usage

```elixir
client = Destack.Client.new()
Destack.Client.backend(client)
Destack.Client.version(client)
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
