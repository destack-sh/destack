require_relative "lib/destack/version"

Gem::Specification.new do |spec|
  spec.name = "destack"
  spec.version = Destack::VERSION
  spec.authors = ["Symbol Industries"]
  spec.email = ["florian@symbol.industries"]

  spec.summary = "Ruby client for Destack"
  spec.description = "Ruby client for Destack"
  spec.homepage = "https://github.com/destack-sh/destack"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/destack-sh/destack"
  spec.metadata["bug_tracker_uri"] = "https://github.com/destack-sh/destack/issues"

  spec.files = Dir["lib/**/*.rb", "ext/**/*.{c,h,rb}"] + ["README.md"]
  spec.extensions = ["ext/destack_ext/extconf.rb"]
  spec.require_paths = ["lib"]
end
