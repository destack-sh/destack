/* generated bridge target, do not edit */

#ifndef DESTACK_ARTIFACT_OUTPUT_GENERATED_H
#define DESTACK_ARTIFACT_OUTPUT_GENERATED_H

#include "destack/core.generated.h"
#include "destack/artifact/version.generated.h"
#include "destack/session/module.generated.h"
#include "destack/source/file.generated.h"
#include "destack/source/product.generated.h"
#include "destack/source/target.generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef enum DestackFileType {
    DESTACK_FILE_TYPE_DESTACK = 0,
    DESTACK_FILE_TYPE_DESTACK_DECLARATION = 1,
    DESTACK_FILE_TYPE_JAVA_SCRIPT = 2,
    DESTACK_FILE_TYPE_JAVA_SCRIPT_XML = 3,
    DESTACK_FILE_TYPE_TYPE_SCRIPT = 4,
    DESTACK_FILE_TYPE_TYPE_SCRIPT_XML = 5,
    DESTACK_FILE_TYPE_TYPE_SCRIPT_DECLARATION = 6,
    DESTACK_FILE_TYPE_TEXT = 7,
    DESTACK_FILE_TYPE_TOML = 8,
    DESTACK_FILE_TYPE_YAML = 9,
    DESTACK_FILE_TYPE_JSON = 10,
    DESTACK_FILE_TYPE_ENV = 11,
    DESTACK_FILE_TYPE_HTML = 12,
    DESTACK_FILE_TYPE_MARKDOWN = 13,
    DESTACK_FILE_TYPE_CSS = 14,
    DESTACK_FILE_TYPE_SVG = 15,
    DESTACK_FILE_TYPE_WASM = 16,
    DESTACK_FILE_TYPE_NODE = 17,
    DESTACK_FILE_TYPE_SOURCE_MAP = 18,
    DESTACK_FILE_TYPE_OBJECT = 19,
    DESTACK_FILE_TYPE_IMAGE = 20,
    DESTACK_FILE_TYPE_FONT = 21,
    DESTACK_FILE_TYPE_AUDIO = 22,
    DESTACK_FILE_TYPE_VIDEO = 23,
    DESTACK_FILE_TYPE_MODEL = 24,
    DESTACK_FILE_TYPE_NEURAL = 25,
    DESTACK_FILE_TYPE_DOCUMENT = 26,
    DESTACK_FILE_TYPE_BINARY = 27,
    DESTACK_FILE_TYPE_UNKNOWN = 28,
} DestackFileType;

typedef struct DestackFileTypeArray {
    DestackFileType *ptr;
    size_t len;
} DestackFileTypeArray;

typedef struct DestackOptionalFileType {
    bool is_some;
    DestackFileType value;
} DestackOptionalFileType;

typedef struct DestackSourceMapSource {
    char *name;
    DestackOptionalString content;
} DestackSourceMapSource;

typedef struct DestackSourceMapSourceArray {
    DestackSourceMapSource *ptr;
    size_t len;
} DestackSourceMapSourceArray;

typedef struct DestackOptionalSourceMapSource {
    bool is_some;
    DestackSourceMapSource value;
} DestackOptionalSourceMapSource;

typedef struct DestackSourceMap {
    uint32_t version;
    DestackOptionalString file;
    DestackOptionalString source_root;
    DestackSourceMapSourceArray sources;
    DestackStringArray names;
    char *mappings;
    DestackOptionalString debug_id;
} DestackSourceMap;

typedef struct DestackSourceMapArray {
    DestackSourceMap *ptr;
    size_t len;
} DestackSourceMapArray;

typedef struct DestackOptionalSourceMap {
    bool is_some;
    DestackSourceMap value;
} DestackOptionalSourceMap;

typedef struct DestackAsset {
    DestackFileType file_type;
    DestackContentId content;
    DestackOptionalString source;
    DestackOptionalSourceMap map;
} DestackAsset;

typedef struct DestackAssetArray {
    DestackAsset *ptr;
    size_t len;
} DestackAssetArray;

typedef struct DestackOptionalAsset {
    bool is_some;
    DestackAsset value;
} DestackOptionalAsset;

typedef enum DestackBuildLinkage {
    DESTACK_BUILD_LINKAGE_PORTABLE = 0,
    DESTACK_BUILD_LINKAGE_STATIC = 1,
    DESTACK_BUILD_LINKAGE_DYNAMIC = 2,
} DestackBuildLinkage;

typedef struct DestackBuildLinkageArray {
    DestackBuildLinkage *ptr;
    size_t len;
} DestackBuildLinkageArray;

typedef struct DestackOptionalBuildLinkage {
    bool is_some;
    DestackBuildLinkage value;
} DestackOptionalBuildLinkage;

typedef enum DestackBuildProfile {
    DESTACK_BUILD_PROFILE_FULL = 0,
    DESTACK_BUILD_PROFILE_MINIMAL = 1,
    DESTACK_BUILD_PROFILE_FREESTANDING = 2,
} DestackBuildProfile;

typedef struct DestackBuildProfileArray {
    DestackBuildProfile *ptr;
    size_t len;
} DestackBuildProfileArray;

typedef struct DestackOptionalBuildProfile {
    bool is_some;
    DestackBuildProfile value;
} DestackOptionalBuildProfile;

typedef struct DestackBuild {
    DestackBuildProfile profile;
    DestackBuildLinkage linkage;
    DestackContentId content;
} DestackBuild;

typedef struct DestackBuildArray {
    DestackBuild *ptr;
    size_t len;
} DestackBuildArray;

typedef struct DestackOptionalBuild {
    bool is_some;
    DestackBuild value;
} DestackOptionalBuild;

typedef enum DestackBundleSection {
    DESTACK_BUNDLE_SECTION_MODULE = 0,
    DESTACK_BUNDLE_SECTION_ENTRY = 1,
    DESTACK_BUNDLE_SECTION_DECLARATION = 2,
    DESTACK_BUNDLE_SECTION_ASSET = 3,
    DESTACK_BUNDLE_SECTION_MANIFEST = 4,
    DESTACK_BUNDLE_SECTION_SOURCE_MAP = 5,
    DESTACK_BUNDLE_SECTION_NATIVE = 6,
} DestackBundleSection;

typedef struct DestackBundleSectionArray {
    DestackBundleSection *ptr;
    size_t len;
} DestackBundleSectionArray;

typedef struct DestackOptionalBundleSection {
    bool is_some;
    DestackBundleSection value;
} DestackOptionalBundleSection;

typedef struct DestackBundleFile {
    DestackBundleSection section;
    char *uri;
    DestackFileType file_type;
    DestackContentId content;
    DestackOptionalString source;
} DestackBundleFile;

typedef struct DestackBundleFileArray {
    DestackBundleFile *ptr;
    size_t len;
} DestackBundleFileArray;

typedef struct DestackOptionalBundleFile {
    bool is_some;
    DestackBundleFile value;
} DestackOptionalBundleFile;

typedef enum DestackBundleMode {
    DESTACK_BUNDLE_MODE_PRESERVE_MODULES = 0,
    DESTACK_BUNDLE_MODE_SINGLE_FILE = 1,
    DESTACK_BUNDLE_MODE_CHUNKED = 2,
} DestackBundleMode;

typedef struct DestackBundleModeArray {
    DestackBundleMode *ptr;
    size_t len;
} DestackBundleModeArray;

typedef struct DestackOptionalBundleMode {
    bool is_some;
    DestackBundleMode value;
} DestackOptionalBundleMode;

typedef enum DestackEmitFormat {
    DESTACK_EMIT_FORMAT_JS = 0,
    DESTACK_EMIT_FORMAT_TS = 1,
    DESTACK_EMIT_FORMAT_WASM = 2,
    DESTACK_EMIT_FORMAT_NATIVE = 3,
} DestackEmitFormat;

typedef struct DestackEmitFormatArray {
    DestackEmitFormat *ptr;
    size_t len;
} DestackEmitFormatArray;

typedef struct DestackOptionalEmitFormat {
    bool is_some;
    DestackEmitFormat value;
} DestackOptionalEmitFormat;

typedef struct DestackBundle {
    DestackEmitFormat emit;
    DestackBundleMode mode;
    DestackBundleFileArray files;
} DestackBundle;

typedef struct DestackBundleArray {
    DestackBundle *ptr;
    size_t len;
} DestackBundleArray;

typedef struct DestackOptionalBundle {
    bool is_some;
    DestackBundle value;
} DestackOptionalBundle;

typedef enum DestackObjectFormat {
    DESTACK_OBJECT_FORMAT_OBJECT = 0,
    DESTACK_OBJECT_FORMAT_WASM = 1,
} DestackObjectFormat;

typedef struct DestackObjectFormatArray {
    DestackObjectFormat *ptr;
    size_t len;
} DestackObjectFormatArray;

typedef struct DestackOptionalObjectFormat {
    bool is_some;
    DestackObjectFormat value;
} DestackOptionalObjectFormat;

typedef struct DestackObject {
    DestackObjectFormat format;
    DestackContentId content;
    DestackOptionalSourceMap map;
} DestackObject;

typedef struct DestackObjectArray {
    DestackObject *ptr;
    size_t len;
} DestackObjectArray;

typedef struct DestackOptionalObject {
    bool is_some;
    DestackObject value;
} DestackOptionalObject;

typedef enum DestackHost {
    DESTACK_HOST_NATIVE = 0,
    DESTACK_HOST_BROWSER = 1,
    DESTACK_HOST_WASI = 2,
    DESTACK_HOST_EMSCRIPTEN = 3,
    DESTACK_HOST_FREESTANDING = 4,
} DestackHost;

typedef struct DestackHostArray {
    DestackHost *ptr;
    size_t len;
} DestackHostArray;

typedef struct DestackOptionalHost {
    bool is_some;
    DestackHost value;
} DestackOptionalHost;

typedef enum DestackRuntime {
    DESTACK_RUNTIME_DESTACK = 0,
    DESTACK_RUNTIME_JS = 1,
} DestackRuntime;

typedef struct DestackRuntimeArray {
    DestackRuntime *ptr;
    size_t len;
} DestackRuntimeArray;

typedef struct DestackOptionalRuntime {
    bool is_some;
    DestackRuntime value;
} DestackOptionalRuntime;

typedef struct DestackProductTarget {
    char *name;
    DestackTargetId target;
    DestackRuntime runtime;
    DestackHost host;
    char *platform;
    bool includes_build;
    bool includes_bundle;
    bool includes_program;
} DestackProductTarget;

typedef struct DestackProductTargetArray {
    DestackProductTarget *ptr;
    size_t len;
} DestackProductTargetArray;

typedef struct DestackOptionalProductTarget {
    bool is_some;
    DestackProductTarget value;
} DestackOptionalProductTarget;

typedef struct DestackProduct {
    char *name;
    DestackProductTargetArray targets;
} DestackProduct;

typedef struct DestackProductArray {
    DestackProduct *ptr;
    size_t len;
} DestackProductArray;

typedef struct DestackOptionalProduct {
    bool is_some;
    DestackProduct value;
} DestackOptionalProduct;

typedef enum DestackProgramFormat {
    DESTACK_PROGRAM_FORMAT_VM = 0,
    DESTACK_PROGRAM_FORMAT_NATIVE = 1,
} DestackProgramFormat;

typedef struct DestackProgramFormatArray {
    DestackProgramFormat *ptr;
    size_t len;
} DestackProgramFormatArray;

typedef struct DestackOptionalProgramFormat {
    bool is_some;
    DestackProgramFormat value;
} DestackOptionalProgramFormat;

typedef struct DestackProgramHeader {
    DestackOptionalString name;
    DestackOptionalString fingerprint;
    DestackOptionalString target;
} DestackProgramHeader;

typedef struct DestackProgramHeaderArray {
    DestackProgramHeader *ptr;
    size_t len;
} DestackProgramHeaderArray;

typedef struct DestackOptionalProgramHeader {
    bool is_some;
    DestackProgramHeader value;
} DestackOptionalProgramHeader;

typedef struct DestackProgram {
    DestackProgramHeader header;
    DestackProgramFormat format;
    DestackContentIdArray contents;
} DestackProgram;

typedef struct DestackProgramArray {
    DestackProgram *ptr;
    size_t len;
} DestackProgramArray;

typedef struct DestackOptionalProgram {
    bool is_some;
    DestackProgram value;
} DestackOptionalProgram;

typedef struct DestackDeclaration {
    char *text;
} DestackDeclaration;

typedef struct DestackDeclarationArray {
    DestackDeclaration *ptr;
    size_t len;
} DestackDeclarationArray;

typedef struct DestackOptionalDeclaration {
    bool is_some;
    DestackDeclaration value;
} DestackOptionalDeclaration;

typedef enum DestackScriptLanguage {
    DESTACK_SCRIPT_LANGUAGE_JAVA_SCRIPT = 0,
    DESTACK_SCRIPT_LANGUAGE_TYPE_SCRIPT = 1,
} DestackScriptLanguage;

typedef struct DestackScriptLanguageArray {
    DestackScriptLanguage *ptr;
    size_t len;
} DestackScriptLanguageArray;

typedef struct DestackOptionalScriptLanguage {
    bool is_some;
    DestackScriptLanguage value;
} DestackOptionalScriptLanguage;

typedef struct DestackScript {
    DestackScriptLanguage language;
    DestackOptionalDeclaration declaration;
    DestackOptionalSourceMap map;
    bool has_top_level_side_effects;
} DestackScript;

typedef struct DestackScriptArray {
    DestackScript *ptr;
    size_t len;
} DestackScriptArray;

typedef struct DestackOptionalScript {
    bool is_some;
    DestackScript value;
} DestackOptionalScript;

typedef enum DestackBuildOutputKind {
    DESTACK_BUILD_OUTPUT_KIND_SCRIPT = 0,
    DESTACK_BUILD_OUTPUT_KIND_OBJECT = 1,
    DESTACK_BUILD_OUTPUT_KIND_ASSET = 2,
    DESTACK_BUILD_OUTPUT_KIND_BUILD = 3,
    DESTACK_BUILD_OUTPUT_KIND_BUNDLE = 4,
    DESTACK_BUILD_OUTPUT_KIND_PROGRAM = 5,
    DESTACK_BUILD_OUTPUT_KIND_PRODUCT = 6,
} DestackBuildOutputKind;

typedef struct DestackBuildOutput {
    DestackBuildOutputKind kind;
    DestackArtifactVersion version;
    DestackScript script;
    DestackObject object;
    DestackAsset asset;
    DestackBuild build;
    DestackBundle bundle;
    DestackProgram program;
    DestackProduct product;
} DestackBuildOutput;

typedef struct DestackBuildOutputArray {
    DestackBuildOutput *ptr;
    size_t len;
} DestackBuildOutputArray;

typedef struct DestackOptionalBuildOutput {
    bool is_some;
    DestackBuildOutput value;
} DestackOptionalBuildOutput;

typedef enum DestackModuleBuildKind {
    DESTACK_MODULE_BUILD_KIND_SCRIPT = 0,
    DESTACK_MODULE_BUILD_KIND_OBJECT = 1,
    DESTACK_MODULE_BUILD_KIND_ASSET = 2,
} DestackModuleBuildKind;

typedef struct DestackModuleBuildKindArray {
    DestackModuleBuildKind *ptr;
    size_t len;
} DestackModuleBuildKindArray;

typedef struct DestackOptionalModuleBuildKind {
    bool is_some;
    DestackModuleBuildKind value;
} DestackOptionalModuleBuildKind;

typedef enum DestackBuildRequestKind {
    DESTACK_BUILD_REQUEST_KIND_MODULE = 0,
    DESTACK_BUILD_REQUEST_KIND_BUILD = 1,
    DESTACK_BUILD_REQUEST_KIND_TARGET = 2,
    DESTACK_BUILD_REQUEST_KIND_PRODUCT = 3,
} DestackBuildRequestKind;

typedef struct DestackBuildRequest {
    DestackBuildRequestKind kind;
    DestackModule module;
    DestackTargetId target;
    DestackModuleBuildKind output;
    DestackProductId product;
} DestackBuildRequest;

typedef struct DestackBuildRequestArray {
    DestackBuildRequest *ptr;
    size_t len;
} DestackBuildRequestArray;

typedef struct DestackOptionalBuildRequest {
    bool is_some;
    DestackBuildRequest value;
} DestackOptionalBuildRequest;

void destack_file_type_destroy(DestackFileType *value);
void destack_file_type_array_destroy(DestackFileTypeArray array);
void destack_source_map_source_destroy(DestackSourceMapSource *value);
void destack_source_map_source_array_destroy(DestackSourceMapSourceArray array);
void destack_source_map_destroy(DestackSourceMap *value);
void destack_source_map_array_destroy(DestackSourceMapArray array);
void destack_asset_destroy(DestackAsset *value);
void destack_asset_array_destroy(DestackAssetArray array);
void destack_build_linkage_destroy(DestackBuildLinkage *value);
void destack_build_linkage_array_destroy(DestackBuildLinkageArray array);
void destack_build_profile_destroy(DestackBuildProfile *value);
void destack_build_profile_array_destroy(DestackBuildProfileArray array);
void destack_build_destroy(DestackBuild *value);
void destack_build_array_destroy(DestackBuildArray array);
void destack_bundle_section_destroy(DestackBundleSection *value);
void destack_bundle_section_array_destroy(DestackBundleSectionArray array);
void destack_bundle_file_destroy(DestackBundleFile *value);
void destack_bundle_file_array_destroy(DestackBundleFileArray array);
void destack_bundle_mode_destroy(DestackBundleMode *value);
void destack_bundle_mode_array_destroy(DestackBundleModeArray array);
void destack_emit_format_destroy(DestackEmitFormat *value);
void destack_emit_format_array_destroy(DestackEmitFormatArray array);
void destack_bundle_destroy(DestackBundle *value);
void destack_bundle_array_destroy(DestackBundleArray array);
void destack_object_format_destroy(DestackObjectFormat *value);
void destack_object_format_array_destroy(DestackObjectFormatArray array);
void destack_object_destroy(DestackObject *value);
void destack_object_array_destroy(DestackObjectArray array);
void destack_host_destroy(DestackHost *value);
void destack_host_array_destroy(DestackHostArray array);
void destack_runtime_destroy(DestackRuntime *value);
void destack_runtime_array_destroy(DestackRuntimeArray array);
void destack_product_target_destroy(DestackProductTarget *value);
void destack_product_target_array_destroy(DestackProductTargetArray array);
void destack_product_destroy(DestackProduct *value);
void destack_product_array_destroy(DestackProductArray array);
void destack_program_format_destroy(DestackProgramFormat *value);
void destack_program_format_array_destroy(DestackProgramFormatArray array);
void destack_program_header_destroy(DestackProgramHeader *value);
void destack_program_header_array_destroy(DestackProgramHeaderArray array);
void destack_program_destroy(DestackProgram *value);
void destack_program_array_destroy(DestackProgramArray array);
void destack_declaration_destroy(DestackDeclaration *value);
void destack_declaration_array_destroy(DestackDeclarationArray array);
void destack_script_language_destroy(DestackScriptLanguage *value);
void destack_script_language_array_destroy(DestackScriptLanguageArray array);
void destack_script_destroy(DestackScript *value);
void destack_script_array_destroy(DestackScriptArray array);
void destack_build_output_destroy(DestackBuildOutput *value);
void destack_build_output_array_destroy(DestackBuildOutputArray array);
void destack_module_build_kind_destroy(DestackModuleBuildKind *value);
void destack_module_build_kind_array_destroy(DestackModuleBuildKindArray array);
void destack_build_request_destroy(DestackBuildRequest *value);
void destack_build_request_array_destroy(DestackBuildRequestArray array);

#ifdef __cplusplus
}
#endif

#endif
