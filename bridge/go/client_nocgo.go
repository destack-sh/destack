//go:build !cgo

package destack

const (
	// Backend is the static backend marker for this package.
	Backend = "go"
	// Version is the package version fallback.
	Version = "0.55.4"
)

// Client is a client type for Destack.
type Client struct{}

// NewClient creates a new client.
func NewClient() Client {
	return Client{}
}

// BackendName returns the backend marker.
func (client Client) BackendName() string {
	return Backend
}

// CapiAbiVersion returns the capi abi version when available.
func (client Client) CapiAbiVersion() uint32 {
	return 0
}

// CapiIsAvailable returns the capi runtime availability.
func (client Client) CapiIsAvailable() bool {
	return false
}

// VersionString returns the package version.
func (client Client) VersionString() string {
	return Version
}
