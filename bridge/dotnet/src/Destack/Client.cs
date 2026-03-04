using System.Runtime.InteropServices;

namespace Destack;

/// <summary>
/// The static backend marker for this package.
/// </summary>
public static class BackendMarker
{
    /// <summary>
    /// The backend name.
    /// </summary>
    public const string Value = "dotnet";
}

/// <summary>
/// A client type for Destack.
/// </summary>
public sealed class Client
{
    private const string FallbackVersion = "0.55.4";

    private static readonly CapiBindings? Capi = CapiBindings.TryLoad();

    /// <summary>
    /// The selected backend marker.
    /// </summary>
    public string Backend => BackendMarker.Value;

    /// <summary>
    /// Return the loaded C ABI version.
    /// </summary>
    public uint CapiAbiVersion()
    {
        return Capi?.AbiVersion() ?? 0;
    }

    /// <summary>
    /// Return whether the C API reports availability.
    /// </summary>
    public bool CapiIsAvailable()
    {
        return Capi?.IsAvailable() ?? false;
    }

    /// <summary>
    /// Return the package version.
    /// </summary>
    public string Version()
    {
        return Capi?.Version() ?? FallbackVersion;
    }
}

internal sealed class CapiBindings
{
    private const string FallbackVersion = "0.55.4";

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint AbiVersionFunction();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate IntPtr VersionFunction();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    private delegate bool IsAvailableFunction();

    private readonly AbiVersionFunction _abiVersion;
    private readonly VersionFunction _version;
    private readonly IsAvailableFunction _isAvailable;

    private CapiBindings(IntPtr handle)
    {
        var abiVersionPointer = NativeLibrary.GetExport(handle, "destack_capi_abi_version");
        var versionPointer = NativeLibrary.GetExport(handle, "destack_capi_version");
        var isAvailablePointer = NativeLibrary.GetExport(handle, "destack_capi_is_available");

        _abiVersion = Marshal.GetDelegateForFunctionPointer<AbiVersionFunction>(abiVersionPointer);
        _version = Marshal.GetDelegateForFunctionPointer<VersionFunction>(versionPointer);
        _isAvailable = Marshal.GetDelegateForFunctionPointer<IsAvailableFunction>(isAvailablePointer);
    }

    public static CapiBindings? TryLoad()
    {
        var explicitPath = Environment.GetEnvironmentVariable("DESTACK_CAPI_LIB");
        var candidates = explicitPath is { Length: > 0 }
            ? new[] { explicitPath }
            : new[] { "destack_capi", "libdestack_capi" };

        foreach (var candidate in candidates)
        {
            if (!NativeLibrary.TryLoad(candidate, out var handle))
            {
                continue;
            }

            try
            {
                return new CapiBindings(handle);
            }
            catch
            {
                // ignore malformed exports and continue searching
            }
        }

        return null;
    }

    public uint AbiVersion()
    {
        return _abiVersion();
    }

    public string Version()
    {
        var pointer = _version();
        if (pointer == IntPtr.Zero)
        {
            return FallbackVersion;
        }

        return Marshal.PtrToStringUTF8(pointer) ?? FallbackVersion;
    }

    public bool IsAvailable()
    {
        return _isAvailable();
    }
}
