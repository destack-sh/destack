package destack

import (
	"testing"
)

// TestClientRoundtrip verifies the baseline client surface.
func TestClientRoundtrip(t *testing.T) {
	client := NewClient()

	if client.BackendName() != Backend {
		t.Fatalf("expected backend %q, got %q", Backend, client.BackendName())
	}

	version := client.VersionString()
	if version == "" {
		t.Fatalf("expected non-empty version")
	}

	abiVersion := client.CapiAbiVersion()
	if abiVersion > 0 && !client.CapiIsAvailable() {
		t.Fatalf("expected capi availability when abi version is set")
	}
}
