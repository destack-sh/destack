import subprocess
from collections import defaultdict
from pathlib import Path

from destack.language import (
    NodeType,
    StructType,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
)

from .const import (
    GENERATION_PATH,
)
from .core import TypescriptDefinition, TypescriptFile
from .encoder import generate_cson_encoders, generate_json_encoders
from .language import (
    BUILTIN_NAMES,
    _generate_constants,
    _generate_definition,
    _generate_file,
    _generate_mapping,
)


def generate():
    """Generate the Typescript SDK."""

    # collect definitions
    definitions_by_module: dict[str, list[TypescriptDefinition]] = defaultdict(list)
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        definition = _generate_definition(ENUM_DEFINITION_BY_TYPE[enum_type])
        definitions_by_module[enum_cls.__module__].append(definition)
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items():
        if struct_type == StructType.STRUCT:
            continue  # manually defined
        definition = _generate_definition(STRUCT_DEFINITION_BY_TYPE[struct_type])
        definitions_by_module[struct_cls.__module__].append(definition)
    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        definition = _generate_definition(TRAIT_DEFINITION_BY_TYPE[trait_type])
        definitions_by_module[trait_cls.__module__].append(definition)
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        if node_type == NodeType.NODE:
            continue  # manually defined
        definition = _generate_definition(NODE_DEFINITION_BY_TYPE[node_type])
        definitions_by_module[node_cls.__module__].append(definition)
    definitions_by_name: dict[str, TypescriptDefinition] = {}
    for _, definitions in definitions_by_module.items():
        for definition in definitions:
            existing_definition = definitions_by_name.get(definition.name)
            if existing_definition is not None:
                raise RuntimeError(
                    f"duplicate definition name: {definition.name} "
                    f"({existing_definition.module}.{existing_definition.alias} vs "
                    f"{definition.module}.{definition.alias})"
                )
            definitions_by_name[definition.name] = definition

    # generate files
    files_by_module: dict[str, TypescriptFile] = {}
    for module, definitions in definitions_by_module.items():
        # path
        target_path = (
            Path(GENERATION_PATH)
            / "language"
            / (module.split(".", 2)[-1].replace(".", "/") + ".ts")
        )
        existing_file_str = target_path.read_text() if target_path.exists() else None

        # definitions
        file_definitions_by_name: dict[str, TypescriptDefinition] = {
            definition.definition.name: definition for definition in definitions
        }
        file_dependencies_by_name: dict[str, TypescriptDefinition] = {}
        for definition in definitions:
            for dependency_name in definition.dependencies:
                if dependency_name not in definitions_by_name:
                    if dependency_name in BUILTIN_NAMES:
                        continue  # manually defined
                    raise RuntimeError(f"missing dependency: {dependency_name}")
                file_dependencies_by_name[dependency_name] = definitions_by_name[dependency_name]

        # file
        file = TypescriptFile(
            name=module,
            module=module,
            path=target_path,
            definitions=file_definitions_by_name,
            dependencies=file_dependencies_by_name,
            existing_str=existing_file_str,
        )
        file.new_str = _generate_file(file, definitions_by_name)
        files_by_module[module] = file

    # write files
    for file in files_by_module.values():
        assert file.new_str, f"empty {file.path}"
        file.path.parent.mkdir(parents=True, exist_ok=True)
        file.path.write_text(file.new_str)

    # write constants file
    constants_path = Path(GENERATION_PATH) / "language" / "constants.ts"
    constants_str = _generate_constants(definitions_by_name)
    constants_path.write_text(constants_str)

    # write mapping files
    mapping_path = Path(GENERATION_PATH) / "language" / "mapping.ts"
    mapping_str = _generate_mapping(definitions_by_name)
    mapping_path.write_text(mapping_str)

    # write encoder files
    cson_encoder_path = Path(GENERATION_PATH) / "encoder/cson/generated.ts"
    cson_encoder_str = generate_cson_encoders()
    cson_encoder_path.write_text(cson_encoder_str)
    json_encoder_path = Path(GENERATION_PATH) / "encoder/json/generated.ts"
    json_encoder_str = generate_json_encoders()
    json_encoder_path.write_text(json_encoder_str)

    # write index files
    module_paths = list({file.path.parent for file in files_by_module.values()})
    module_paths.append(Path(GENERATION_PATH) / "language")
    for module_path in sorted(module_paths):
        # generate index.ts file that re-exports every subfile/subfolder
        index_path = module_path / "index.ts"
        index_path.parent.mkdir(parents=True, exist_ok=True)
        index_lines = []
        # collect all .ts files in this directory (excluding index.ts itself)
        ts_files = [f for f in module_path.glob("*.ts") if f.name != "index.ts"]
        for ts_file in sorted(ts_files):
            relative_path = ts_file.relative_to(Path(GENERATION_PATH) / "language").with_suffix("")
            index_lines.append(f"export * from '@destack/language/{relative_path}';")
        # collect all subdirectories that contain .ts files
        for subdir in sorted(module_path.iterdir()):
            if subdir.is_dir() and any(subdir.rglob("*.ts")):
                relative_path = subdir.relative_to(Path(GENERATION_PATH) / "language").with_suffix(
                    ""
                )
                index_lines.append(f"export * from '@destack/language/{relative_path}';")

        index_content = "\n".join(index_lines) + "\n"
        index_path.write_text(index_content)

    # append finalize call to root index.ts
    root_index_path = Path(GENERATION_PATH) / "language" / "index.ts"
    root_index_content = (
        root_index_path.read_text()
        + """
import { finalize } from "@destack/language/finalize";
finalize();
"""
    )
    root_index_path.write_text(root_index_content)

    # format it all
    subprocess.run("cd destack-ts && bun run fmt", shell=True, check=True)
