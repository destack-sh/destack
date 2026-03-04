import "dart:ffi";
import "dart:io";

import "package:ffi/ffi.dart";

class DestackClient {
  static const String backendMarker = "dart";
  static const String packageVersion = "0.55.3";

  static final _capi = _DestackCapi.tryLoad();

  String get backend => backendMarker;

  int capiAbiVersion() {
    return _capi?.abiVersion() ?? 0;
  }

  bool capiIsAvailable() {
    return _capi?.isAvailable() ?? false;
  }

  String version() {
    return _capi?.version() ?? packageVersion;
  }
}

class _DestackCapi {
  _DestackCapi(this._abiVersion, this._version, this._isAvailable);

  final int Function() _abiVersion;
  final Pointer<Utf8> Function() _version;
  final int Function() _isAvailable;

  static _DestackCapi? tryLoad() {
    final explicitPath = Platform.environment["DESTACK_CAPI_LIB"];
    final candidates = <String>[
      if (explicitPath != null && explicitPath.isNotEmpty) explicitPath,
      if (Platform.isMacOS) "libdestack_capi.dylib",
      if (Platform.isLinux) "libdestack_capi.so",
      if (Platform.isWindows) "destack_capi.dll",
    ];

    for (final candidate in candidates) {
      try {
        final library = DynamicLibrary.open(candidate);
        final abiVersion = library.lookupFunction<Uint32 Function(), int Function()>(
          "destack_capi_abi_version",
        );
        final version = library.lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
          "destack_capi_version",
        );
        final isAvailable = library.lookupFunction<Uint8 Function(), int Function()>(
          "destack_capi_is_available",
        );
        return _DestackCapi(abiVersion, version, isAvailable);
      } on Object {
        // keep searching
      }
    }

    return null;
  }

  int abiVersion() {
    return _abiVersion();
  }

  bool isAvailable() {
    return _isAvailable() != 0;
  }

  String version() {
    final value = _version();
    if (value == nullptr) {
      return DestackClient.packageVersion;
    }

    final text = value.toDartString();
    if (text.isEmpty) {
      return DestackClient.packageVersion;
    }

    return text;
  }
}
