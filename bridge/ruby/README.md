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
