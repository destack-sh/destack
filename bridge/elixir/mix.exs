defmodule Destack.MixProject do
  use Mix.Project

  def project do
    [
      app: :destack,
      version: "0.55.3",
      elixir: "~> 1.16",
      start_permanent: Mix.env() == :prod,
      deps: [],
      description: "Elixir client for Destack",
      package: package(),
      source_url: "https://github.com/destack-sh/destack"
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp package do
    [
      licenses: ["MIT"],
      links: %{
        "GitHub" => "https://github.com/destack-sh/destack"
      }
    ]
  end
end
