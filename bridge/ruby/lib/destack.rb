require_relative "destack/version"
require_relative "destack/client"

module Destack
  begin
    require "destack_ext"
  rescue LoadError => error
    NATIVE_LOAD_ERROR = error
  end

  def self.native_extension_available?
    defined?(Native)
  end

  def self.native_load_error
    return nil unless const_defined?(:NATIVE_LOAD_ERROR, false)

    NATIVE_LOAD_ERROR
  end
end
