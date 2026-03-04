defmodule Destack do
  @moduledoc """
  Package constants for Destack.
  """

  @backend "elixir"
  @version "0.55.4"

  @doc """
  Return the backend marker.
  """
  def backend, do: @backend

  @doc """
  Return the package version.
  """
  def version do
    native_version = Destack.Native.capi_version()

    if is_binary(native_version) and native_version != "" do
      native_version
    else
      @version
    end
  end

  @doc """
  Return the loaded capi abi version.
  """
  def capi_abi_version do
    Destack.Native.capi_abi_version()
  end

  @doc """
  Return whether the capi surface is available.
  """
  def capi_is_available do
    Destack.Native.capi_is_available()
  end

  @doc """
  Return the nif load error when native bindings are unavailable.
  """
  def native_load_error do
    Destack.Native.load_error()
  end
end
