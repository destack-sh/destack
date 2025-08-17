import importlib
from collections import defaultdict
from collections.abc import Mapping
from itertools import chain
from pathlib import Path
from typing import TYPE_CHECKING, Any, Callable, cast

from .core.builtin import (
    VERSION,
    ActionDeclaration,
    EnumType,
    FunctionDeclaration,
    HandleType,
    MethodDeclaration,
    NodeType,
    StructType,
    UniverseCategory,
    UniverseDomain,
)
from .registry import (
    BUILTIN_CLASS_BY_NAME,
    BUILTIN_DEFINITION_BY_NAME,
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    HANDLE_CLASS_BY_TYPE,
    HANDLE_DEFINITION_BY_TYPE,
    MODULE_BY_PATH,
    MODULE_DEFINITION_BY_CATEGORY,
    MODULE_DEFINITION_BY_DOMAIN,
    MODULE_DEFINITION_BY_PATH,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    OBJECT_DEFINITION_REFERENCE_BY_CLASS,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)

if TYPE_CHECKING:
    from destack import (
        ActionDefinition,
        MethodDefinition,
        ModuleDefinition,
        Node,
        SchemaDefinition,
        Struct,
    )

type_ = type


def _index_inheritance(cls_by_type: Mapping[Any, type["Struct | Node"]]):
    # index Object extended_by (direct) / inherited_by (direct and indirect)
    object_type_extended_by: dict[int, list[int]] = defaultdict(list)
    for object_cls in cls_by_type.values():
        if object_cls.__declaration__.base_type is not None:
            object_type_extended_by[object_cls.__declaration__.base_type].append(
                object_cls.metatype
            )
    for object_cls in cls_by_type.values():
        object_cls.__declaration__.extended_by = list(object_type_extended_by[object_cls.metatype])  # type: ignore

    # index Object inherited_by (recursive)
    for object_cls in cls_by_type.values():
        # collect all types that inherit from this object recursively
        inherited_by_objects: list[int] = []
        to_visit = list(reversed(object_cls.__declaration__.extended_by))
        while to_visit:
            inheriting_type = to_visit.pop()
            if inheriting_type not in inherited_by_objects:
                inherited_by_objects.append(inheriting_type)
                inheriting_cls = cls_by_type[inheriting_type]
                to_visit.extend(reversed(inheriting_cls.__declaration__.extended_by))
        object_cls.__declaration__.inherited_by = list(inherited_by_objects)  # type: ignore


_IGNORED_MODULES = (
    "venv",
    "test",
    "requirements",
    "registry",
    "finalize",
    "pytest_cache",
)
_IGNORED_MODULES_EXTENSIONS = (".pyc", ".egg-info")

_hoisted_objects: dict[str, Any] = {}


def _index_module(
    path: str, file_path: Path, parent: "ModuleDefinition | None"
) -> "ModuleDefinition":
    """Collect ModuleDefinitions recursively from filesystem."""

    from .core import (
        ConstantDeclaration,
        ConstantDefinition,
        Enum,
        FlagEnum,
        Handle,
        ModuleDefinition,
        ModuleType,
        Node,
        OptionEnum,
        Struct,
    )

    subname = path.split(".")[-1]

    # identify type/domain/category (if not root)
    type: ModuleType
    domain: UniverseDomain | None
    category: UniverseCategory | None
    if parent is not None:
        domain = parent.domain
        category = parent.category
        if parent.domain is None:
            type = ModuleType.DOMAIN
            domain = UniverseDomain[subname.upper()]
        elif parent.category is None:
            type = ModuleType.CATEGORY
            category = UniverseCategory[subname.upper()]
        else:
            type = ModuleType.OBJECT
    else:
        type = ModuleType.ROOT
        domain = None
        category = None

    # load module
    methods: list[MethodDefinition] = []
    node_types: list[NodeType] = []
    struct_types: list[StructType] = []
    handle_types: list[HandleType] = []
    enum_types: list[EnumType] = []
    constants: list[ConstantDefinition] = []
    if file_path.is_file():
        py_module = importlib.import_module(path)
        description = py_module.__doc__ or ""

        def _index_type(obj: type_):
            if issubclass(obj, Node):
                assert obj.__definition__.domain == domain, (
                    f"node {obj.__name__} has domain {obj.__definition__.domain} but is in {domain} ({path})"
                )
                assert obj.__definition__.category == category, (
                    f"node {obj.__name__} has category {obj.__definition__.category} but is in {category} ({path})"
                )
                node_types.append(obj.metatype)
            elif issubclass(obj, Struct):
                assert obj.__definition__.domain == domain, (
                    f"struct {obj.__name__} has domain {obj.__definition__.domain} but is in {domain} ({path})"
                )
                assert obj.__definition__.category == category, (
                    f"struct {obj.__name__} has category {obj.__definition__.category} but is in {category} ({path})"
                )
                struct_types.append(obj.metatype)
            elif issubclass(obj, Handle):
                assert obj.__definition__.domain == domain, (
                    f"handle {obj.__name__} has domain {obj.__definition__.domain} but is in {domain} ({path})"
                )
                assert obj.__definition__.category == category, (
                    f"handle {obj.__name__} has category {obj.__definition__.category} but is in {category} ({path})"
                )
                handle_types.append(obj.metatype)
            elif issubclass(obj, Enum) and obj not in (Enum, OptionEnum, FlagEnum):
                assert obj.__declaration__.domain == domain, (
                    f"enum {obj.__name__} has domain {obj.__declaration__.domain} but is in {domain} ({path})"
                )
                assert obj.__declaration__.category == category, (
                    f"enum {obj.__name__} has category {obj.__declaration__.category} but is in {category} ({path})"
                )
                enum_types.append(obj.metatype)

        # regular objects
        for obj_name, obj in py_module.__dict__.items():
            if obj_name.startswith("_"):
                continue  # internal object
            elif isinstance(obj, MethodDeclaration):
                MethodDefinition_ = cast(
                    type_["MethodDefinition"], STRUCT_CLASS_BY_TYPE[StructType.METHOD_DEFINITION]
                )
                method = MethodDefinition_.from_declaration(obj)
                methods.append(method)
            elif isinstance(obj, type_) and obj.__module__ == py_module.__name__:
                _index_type(obj)

        # constants from special __constants__ attribute
        if declared_constants := py_module.__dict__.get("__constants__"):
            for constant in declared_constants:
                assert isinstance(constant, ConstantDeclaration), (
                    f"invalid constant in {path}: {constant}"
                )
                constant = ConstantDefinition.from_declaration(constant)
                assert not constant._is_deferred, f"deferred constant in {path}: {constant!r}"
                constants.append(constant)

        # add any extra "hoisted" objects to first leaf module
        #  (which won't show up in its own __dict__ because they're .. hoisted)
        if category is not None:
            hoisted_module_path = Path("destack.core.builtin._hoisted")
            hoisted_module = importlib.import_module(str(hoisted_module_path))
            for obj_name, obj in hoisted_module.__dict__.items():
                if obj_name.startswith("_"):
                    continue  # internal object
                elif (
                    isinstance(obj, type_)
                    and obj.__module__ == hoisted_module_path.name
                    and (declaration := getattr(obj, "__declaration__", None)) is not None
                    and declaration.domain == domain
                    and declaration.category == category
                    and obj_name not in _hoisted_objects
                ):
                    _hoisted_objects[obj_name] = obj
                    _index_type(obj)

        MODULE_BY_PATH[path] = py_module
    else:
        description = ""

    # create module
    module = ModuleDefinition(
        # meta
        type=type,
        name=path,
        path=path,
        domain=domain,
        category=category,
        description=description,
        # content
        methods=sorted(methods, key=lambda m: m.id),
        constants=sorted(constants, key=lambda c: c.id),
        node_types=sorted(node_types),
        struct_types=sorted(struct_types),
        handle_types=sorted(handle_types),
        enum_types=sorted(enum_types),
        # graph
        parent_path=parent.path if parent is not None else None,
        children_paths=[],
    )

    # index
    if (existing_module := MODULE_DEFINITION_BY_PATH.get(path)) is not None:
        raise ValueError(f"duplicate module: {path} ({existing_module} != {module})")
    MODULE_DEFINITION_BY_PATH[path] = module
    if type == ModuleType.DOMAIN:
        assert domain is not None, f"no domain for {path}"
        if (existing_module := MODULE_DEFINITION_BY_DOMAIN.get(domain)) is not None:
            raise ValueError(f"duplicate domain: {domain} ({existing_module} != {module})")
        MODULE_DEFINITION_BY_DOMAIN[domain] = module
    elif type == ModuleType.CATEGORY:
        assert category is not None, f"no category for {path}"
        if (existing_module := MODULE_DEFINITION_BY_CATEGORY.get(category)) is not None:
            raise ValueError(f"duplicate category: {category} ({existing_module} != {module})")
        MODULE_DEFINITION_BY_CATEGORY[category] = module

    # walk children
    if file_path.is_dir():
        children_paths: list[str] = []
        for child_file_path in file_path.iterdir():
            if (
                child_file_path.name.startswith("_")
                or child_file_path.name.startswith(".")
                or child_file_path.stem in _IGNORED_MODULES
                or child_file_path.suffix in _IGNORED_MODULES_EXTENSIONS
            ):
                continue
            if (child_file_path.is_file() and child_file_path.suffix == ".py") or (
                child_file_path.is_dir()
            ):
                child_path = path + "." + child_file_path.stem
                child_module = _index_module(child_path, child_file_path, module)
                children_paths.append(child_module.path)

        module.children_paths = children_paths

    return module


def _make_schema() -> "SchemaDefinition":
    """Make the actual Destack SchemaDefinition struct."""

    from .core import SchemaDefinition
    from .core.builtin.object import _is_finalized

    if not _is_finalized():
        raise RuntimeError("Destack not finalized")

    return SchemaDefinition(
        name="Destack",
        description="The Destack specification",
        version=VERSION,
        nodes=list(NODE_DEFINITION_BY_TYPE.values()),
        structs=list(STRUCT_DEFINITION_BY_TYPE.values()),
        handles=list(HANDLE_DEFINITION_BY_TYPE.values()),
        enums=list(ENUM_DEFINITION_BY_TYPE.values()),
        modules=list(MODULE_DEFINITION_BY_PATH.values()),
    )


def finalize():
    """Finalize the Destack schema."""
    from .core.builtin.object import _is_finalized, _set_finalized

    if _is_finalized():
        return

    from .core import (
        ConstantDeclaration,
        ConstantDefinition,
        EnumDefinition,
        Event,
        HandleDefinition,
        Node,
        NodeDefinition,
        ObjectDefinitionReference,
        PropertyDeclaration,
        StructDefinition,
    )

    # index builtin classes by name
    for cls in chain(
        NODE_CLASS_BY_TYPE.values(),
        STRUCT_CLASS_BY_TYPE.values(),
        ENUM_CLASS_BY_TYPE.values(),
        HANDLE_CLASS_BY_TYPE.values(),
    ):
        if cls.__name__ in BUILTIN_CLASS_BY_NAME:
            existing_cls = BUILTIN_CLASS_BY_NAME[cls.__name__]
            if existing_cls is not cls:
                raise ValueError(f"duplicate class name: {cls.__name__!r}")
        BUILTIN_CLASS_BY_NAME[cls.__name__] = cls

    # index inheritance
    _index_inheritance(NODE_CLASS_BY_TYPE)
    _index_inheritance(STRUCT_CLASS_BY_TYPE)

    # index Node parent types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__declaration__.parent_property is None:
            continue
        elif node_cls.metatype == NodeType.SPACE:
            node_cls.__declaration__.parent_types = []
            node_cls.__declaration__.parent_property.type.node_types = ()
        else:
            parent_types = node_cls.__declaration__.parent_property.type.node_types or ()
            assert len(parent_types) < len(NodeType), f"generic parent for '{node_cls.__name__}'"
            node_cls.__declaration__.parent_types = list(parent_types)

    # index child types
    child_types_by_parent: dict[NodeType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for parent_type in node_cls.__declaration__.parent_types:
            child_types_by_parent[parent_type].append(node_cls.metatype)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_cls.__declaration__.child_types = list(child_types_by_parent[node_cls.metatype])

    # index ancestor/descendant types (recursive)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        # collect all ancestor types recursively
        ancestors: list[NodeType] = []
        to_visit = list(node_cls.__declaration__.parent_types)
        while to_visit:
            parent_type = to_visit.pop()
            if parent_type not in ancestors:
                ancestors.append(parent_type)
                parent_cls = NODE_CLASS_BY_TYPE[parent_type]
                to_visit.extend(parent_cls.__declaration__.parent_types)

        # collect all descendant types recursively
        descendants: list[NodeType] = []
        to_visit = list(node_cls.__declaration__.child_types)
        while to_visit:
            child_type = to_visit.pop()
            if child_type not in descendants:
                descendants.append(child_type)
                child_cls = NODE_CLASS_BY_TYPE[child_type]
                to_visit.extend(child_cls.__declaration__.child_types)

        node_cls.__declaration__.ancestor_types = list(ancestors)
        node_cls.__declaration__.descendant_types = list(descendants)

    # index Node event types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        all_event_types: set[NodeType] = set()
        for base in node_cls.__bases__:
            if issubclass(base, Node) and base.__declaration__.self_event_types:
                all_event_types.update(base.__declaration__.self_event_types)
        node_cls.__declaration__.event_types = list(all_event_types)

    # generate definition refs
    for node_cls in NODE_CLASS_BY_TYPE.values():
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[node_cls] = ObjectDefinitionReference.of(node_cls)
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[struct_cls] = ObjectDefinitionReference.of(struct_cls)

    # generate meta info
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_definition = NodeDefinition.from_declaration(node_cls, node_cls.__declaration__)
        NODE_DEFINITION_BY_TYPE[node_cls.metatype] = node_definition
        node_cls.__definition__ = node_definition
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_definition = StructDefinition.from_declaration(
            struct_cls, struct_cls.__declaration__
        )
        STRUCT_DEFINITION_BY_TYPE[struct_cls.metatype] = struct_definition
        struct_cls.__definition__ = struct_definition
    for handle_cls in HANDLE_CLASS_BY_TYPE.values():
        handle_definition = HandleDefinition.from_declaration(handle_cls.__declaration__)
        HANDLE_DEFINITION_BY_TYPE[handle_cls.metatype] = handle_definition
        handle_cls.__definition__ = handle_definition
    for enum_cls in ENUM_CLASS_BY_TYPE.values():
        enum_definition = EnumDefinition.from_declaration(enum_cls.__declaration__)
        ENUM_DEFINITION_BY_TYPE[enum_cls.metatype] = enum_definition

    # collect methods/actions from objects
    MethodDefinition_ = cast(
        type["MethodDefinition"], STRUCT_CLASS_BY_TYPE[StructType.METHOD_DEFINITION]
    )
    ActionDefinition_ = cast(
        type["ActionDefinition"], STRUCT_CLASS_BY_TYPE[StructType.ACTION_DEFINITION]
    )
    for object_cls in chain(
        NODE_CLASS_BY_TYPE.values(),
        STRUCT_CLASS_BY_TYPE.values(),
        HANDLE_CLASS_BY_TYPE.values(),
    ):
        methods: list[MethodDefinition] = []
        actions: list[ActionDefinition] = []
        for name, attribute in object_cls.__dict__.items():
            if isinstance(attribute, FunctionDeclaration):
                if isinstance(attribute, MethodDeclaration):
                    method = MethodDefinition_.from_declaration(attribute)
                    methods.append(method)
                elif isinstance(attribute, ActionDeclaration):
                    action = ActionDefinition_.from_declaration(attribute)
                    actions.append(action)
                else:
                    raise ValueError(f"unexpected function: {name!r}")
        if methods:
            object_cls.__definition__.methods = list(methods)  # type: ignore
        if actions:
            object_cls.__definition__.actions = list(actions)  # type: ignore

    # finalize constants from objects
    for object_cls in chain(
        NODE_CLASS_BY_TYPE.values(),
        STRUCT_CLASS_BY_TYPE.values(),
        HANDLE_CLASS_BY_TYPE.values(),
    ):
        constants: list[ConstantDefinition] = []
        for name, attribute in object_cls.__dict__.items():
            if isinstance(attribute, ConstantDeclaration):
                if isinstance(attribute.value, Callable):
                    attribute.value = attribute.value()
                constant = ConstantDefinition.from_declaration(attribute)
                constants.append(constant)
                # replace constant with value
                setattr(object_cls, name, attribute.value)
        object_cls.__definition__.constants = list(constants)

    # collect modules
    root_module_path = Path(__file__).parent
    _ = _index_module("destack", root_module_path, parent=None)

    # check that the modules are complete
    #  (should cover BUILTIN_CLASS_BY_NAME completely)
    module_objects_by_name: dict[str, Any] = {}
    for module in MODULE_DEFINITION_BY_PATH.values():
        for node_type in module.node_types:
            node_cls = NODE_CLASS_BY_TYPE[node_type]
            module_objects_by_name[node_cls.__name__] = node_cls
        for struct_type in module.struct_types:
            struct_cls = STRUCT_CLASS_BY_TYPE[struct_type]
            module_objects_by_name[struct_cls.__name__] = struct_cls
        for handle_type in module.handle_types:
            handle_cls = HANDLE_CLASS_BY_TYPE[handle_type]
            module_objects_by_name[handle_cls.__name__] = handle_cls
        for enum_type in module.enum_types:
            enum_cls = ENUM_CLASS_BY_TYPE[enum_type]
            module_objects_by_name[enum_cls.__name__] = enum_cls
    if len(module_objects_by_name) != len(BUILTIN_CLASS_BY_NAME):
        missing_objects = (
            set(BUILTIN_CLASS_BY_NAME.keys()) - set(module_objects_by_name.keys())
        ) | (set(module_objects_by_name.keys()) - set(BUILTIN_CLASS_BY_NAME.keys()))
        raise ValueError(
            f"missing {len(missing_objects)} objects in {len(MODULE_DEFINITION_BY_PATH)} modules: {list(missing_objects)}"
        )

    # index definitions by name
    for definition in chain(
        NODE_DEFINITION_BY_TYPE.values(),
        STRUCT_DEFINITION_BY_TYPE.values(),
        HANDLE_DEFINITION_BY_TYPE.values(),
        ENUM_DEFINITION_BY_TYPE.values(),
        MODULE_DEFINITION_BY_PATH.values(),
    ):
        if (existing_definition := BUILTIN_DEFINITION_BY_NAME.get(definition.name)) is not None:
            raise ValueError(
                f"duplicate definition name for {definition.name!r}: {definition!r} != {existing_definition!r}"
            )
        BUILTIN_DEFINITION_BY_NAME[definition.name] = definition

    # impute methods/actions in classes
    for object_cls in chain(
        NODE_CLASS_BY_TYPE.values(),
        STRUCT_CLASS_BY_TYPE.values(),
        HANDLE_CLASS_BY_TYPE.values(),
        MODULE_BY_PATH.values(),
    ):
        for name, attribute in object_cls.__dict__.items():
            if isinstance(attribute, FunctionDeclaration):
                # replace function with method/action
                setattr(object_cls, name, attribute.outer_func)

    #
    # Validate
    #

    # check that we have a module for each domain and category
    if len(UniverseDomain) != len(MODULE_DEFINITION_BY_DOMAIN):
        missing_domains = set(UniverseDomain) - set(MODULE_DEFINITION_BY_DOMAIN.keys())
        raise ValueError(
            f"missing {len(missing_domains)} UniverseDomain modules: {list(missing_domains)}"
        )
    if len(UniverseCategory) != len(MODULE_DEFINITION_BY_CATEGORY):
        missing_categories = set(UniverseCategory) - set(MODULE_DEFINITION_BY_CATEGORY.keys())
        raise ValueError(
            f"missing {len(missing_categories)} UniverseCategory modules: {list(missing_categories)}"
        )

    # check we have all the declared builtin objects
    if len(EnumType) != len(ENUM_CLASS_BY_TYPE):
        missing_enum_types = set(EnumType) - set(ENUM_CLASS_BY_TYPE.keys())
        raise ValueError(
            f"missing {len(missing_enum_types)} Enum declarations: {list(missing_enum_types)}"
        )
    if len(StructType) != len(STRUCT_CLASS_BY_TYPE):
        missing_struct_types = set(StructType) - set(STRUCT_CLASS_BY_TYPE.keys())
        raise ValueError(
            f"missing {len(missing_struct_types)} Struct declarations: {list(missing_struct_types)}"
        )
    if len(NodeType) != len(NODE_CLASS_BY_TYPE):
        missing_node_types = set(NodeType) - set(NODE_CLASS_BY_TYPE.keys())
        raise ValueError(
            f"missing {len(missing_node_types)} Node declarations: {list(missing_node_types)}"
        )
    if len(HandleType) != len(HANDLE_CLASS_BY_TYPE):
        missing_handle_types = set(HandleType) - set(HANDLE_CLASS_BY_TYPE.keys())
        raise ValueError(
            f"missing {len(missing_handle_types)} Handle declarations: {list(missing_handle_types)}"
        )

    # check event types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if issubclass(node_cls, Event):
            assert not node_cls.__declaration__.event_types, (
                f"{node_cls.__name__} is an Event but has event types: {node_cls.__declaration__.event_types}"
            )

    # check parent types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__declaration__.parent_property is None:
            continue
        # check if parent is compatible with bases
        parent_node_types = node_cls.__declaration__.parent_property.type.node_types or ()
        for base_cls in node_cls.__bases__:
            if isinstance(
                base_parent_property := getattr(base_cls, "__parent_property__", None),
                PropertyDeclaration,
            ):
                base_parent_node_types = base_parent_property.type.node_types or ()
                if (
                    NodeType.NODE in base_parent_node_types
                    or NodeType.ENTITY in base_parent_node_types
                ):
                    continue  # covers everything
                missing_base_node_types: list[NodeType] = []
                for parent_node_type in parent_node_types:
                    if not any(
                        issubclass(
                            NODE_CLASS_BY_TYPE[parent_node_type],
                            NODE_CLASS_BY_TYPE[base_parent_node_type],
                        )
                        for base_parent_node_type in base_parent_node_types
                    ):
                        missing_base_node_types.append(parent_node_type)
                if missing_base_node_types:
                    raise ValueError(
                        f"{node_cls.__name__}.parent is not compatible with {base_cls.__name__}.parent (missing {[t.name for t in missing_base_node_types]})"
                    )

    _set_finalized()


finalize()


SCHEMA = _make_schema()
