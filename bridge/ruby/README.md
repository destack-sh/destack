# destack (Ruby)

Ruby client for Destack.
This gem is intended to publish as `destack`.
The client uses a native extension to call the shared `destack_capi` library when available.

## Usage

```ruby
require "destack"

client = Destack::Client.new
puts client.backend
puts client.version
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
