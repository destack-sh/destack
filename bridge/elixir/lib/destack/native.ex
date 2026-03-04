defmodule Destack.Native do
  @moduledoc false

  require Logger

  @on_load :load_nif
  @load_error_key {__MODULE__, :load_error}

  def load_nif do
    explicit_path = System.get_env("DESTACK_NIF_PATH")
    library_path =
      if is_binary(explicit_path) and explicit_path != "" do
        explicit_path
      else
        Path.join(:code.priv_dir(:destack), "destack_nif")
      end

    case :erlang.load_nif(to_charlist(library_path), 0) do
      :ok ->
        :persistent_term.erase(@load_error_key)
        :ok

      {:error, reason} ->
        :persistent_term.put(@load_error_key, reason)
        Logger.warning("destack nif unavailable: #{inspect(reason)}")
        :ok
    end
  end

  def load_error, do: :persistent_term.get(@load_error_key, nil)

  def capi_abi_version, do: 0

  def capi_is_available, do: false

  def capi_version, do: "0.55.4"
end
