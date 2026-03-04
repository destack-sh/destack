defmodule Destack.Client do
  @moduledoc """
  A client type for Destack.
  """

  defstruct []

  @doc """
  Create a new client.
  """
  def new do
    %__MODULE__{}
  end

  @doc """
  Return the backend marker.
  """
  def backend(%__MODULE__{}), do: Destack.backend()

  @doc """
  Return the package version.
  """
  def version(%__MODULE__{}), do: Destack.version()

  @doc """
  Return the loaded capi abi version.
  """
  def capi_abi_version(%__MODULE__{}), do: Destack.capi_abi_version()

  @doc """
  Return whether the capi surface is available.
  """
  def capi_is_available(%__MODULE__{}), do: Destack.capi_is_available()
end
