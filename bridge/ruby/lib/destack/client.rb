module Destack
  class Client
    def backend
      BACKEND
    end

    def capi_abi_version
      return 0 unless Destack.native_extension_available?

      Destack::Native.capi_abi_version
    end

    def capi_is_available
      return false unless Destack.native_extension_available?

      Destack::Native.capi_is_available
    end

    def version
      return VERSION unless Destack.native_extension_available?

      native_version = Destack::Native.capi_version
      return VERSION if native_version.nil? || native_version.empty?

      native_version
    end
  end
end
