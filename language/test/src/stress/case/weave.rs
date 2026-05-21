use super::StressMode;
use super::generator::{Generator, emit};

/// Generate interwoven TypeScript-shaped source.
pub(super) fn woven_source_forms(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 780);
    generator.emit("export interface WovenInput<T> {\n");
    generator.emit("    readonly items: readonly T[];\n");
    generator.emit("    select(index: number): T | undefined;\n");
    generator.emit("}\n\n");
    generator.emit("export class WovenStore<T extends { id: string }> {\n");
    generator.emit("    constructor(readonly input: WovenInput<T>) {}\n\n");

    for index in 0..generator.scale() {
        emit_source_method(&mut generator, index);
    }

    generator.emit("}\n\n");
    emit_width_marker(&mut generator);

    for index in 0..generator.scale() {
        emit_source_expression(&mut generator, index);
    }

    generator.finish()
}

/// Generate interwoven Destack source.
pub(super) fn woven_destack_forms(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 1_200);
    generator.emit("import config from \"./stress.toml\" with { type: \"json\" };\n\n");
    generator.emit("@noHeap\nmodule {\n");
    generator.emit("    const product = \"stress\";\n");
    generator.emit("    const tags = [\"parser\", \"formatter\"];\n");
    generator.emit("}\n\n");
    generator.emit("type ByteSlice = [uint8];\n");
    generator.emit("type ByteBlock<comptime N: uint> = [uint8; N];\n");
    generator.emit("type Digit = 0..=9;\n");
    generator.emit("newtype PacketId = uint64;\n\n");
    emit_width_marker(&mut generator);

    for index in 0..generator.scale() {
        emit_destack_declaration(&mut generator, index);
        emit_destack_function(&mut generator, index);
    }

    generator.finish()
}

/// Generate interwoven TSX source.
pub(super) fn woven_tsx_forms(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 780);

    if generator.mode() == StressMode::Destack {
        generator.emit("const WovenTree = ");
    } else {
        generator.emit("export const WovenTree = ");
    }

    generator.emit("<Panel title=\"stress\">\n");

    for index in 0..generator.scale() {
        emit_tsx_section(&mut generator, index);
    }

    generator.emit("</Panel>;\n\n");
    emit_width_marker(&mut generator);
    generator.emit(
        "const WovenSummary = items.map((item) => ({ id: item.id, label: item.label ?? \"missing\" }));\n",
    );

    generator.finish()
}

/// Emit a width dependent declaration.
fn emit_width_marker(generator: &mut Generator) {
    emit!(
        generator,
        "const stressWidth = {} satisfies number;\n\n",
        generator.width()
    );
}

/// Write one generic class method.
fn emit_source_method(generator: &mut Generator, index: usize) {
    emit!(
        generator,
        "    read{index}<Value extends T>(index: number, fallback: Value): Value {{\n"
    );
    generator.emit("        const selected = this.input.select(index);\n");
    generator.emit("        return selected ?? fallback;\n");
    generator.emit("    }\n\n");
}

/// Write one expression/object pair.
fn emit_source_expression(generator: &mut Generator, index: usize) {
    emit!(
        generator,
        "const wovenValue{index} = store.read{index}({index}, fallback).id satisfies string;\n"
    );
    emit!(
        generator,
        "const wovenObject{index} = {{ id: wovenValue{index}, meta: {{ index: {index} }} }};\n"
    );
}

/// Write one decorated nominal declaration.
fn emit_destack_declaration(generator: &mut Generator, index: usize) {
    generator.emit("@derive(Clone, Debug)\n");
    emit!(generator, "struct Packet{index}<comptime Size: uint> {{\n");
    generator.emit("    comptime {\n");
    generator.emit("        assert(Size > 0);\n");
    generator.emit("    }\n");
    generator.emit("    id: PacketId;\n");
    generator.emit("    data: [uint8; Size];\n");
    generator.emit("}\n\n");
}

/// Write one mixed Destack function.
fn emit_destack_function(generator: &mut Generator, index: usize) {
    let end = index + 4;
    emit!(
        generator,
        "function parsePacket{index}(bytes: ByteSlice): Result<uint8, IOError> {{\n"
    );
    generator.emit("    using trace = openTrace(config.tracePath)?;\n");
    generator.emit("    const header = bytes[0..4];\n");
    emit!(generator, "    const window = bytes[{index}..={end}];\n");
    generator.emit("    return match (header) {\n");
    generator.emit("        [0x89, 0x50, 0x4e, 0x47] => window[0]\n");
    generator.emit("        [first, second, ...rest] if (first == second) => rest[0]\n");
    generator.emit("        _ => Result.err(IOError.InvalidPacket)\n");
    generator.emit("    };\n");
    generator.emit("}\n\n");
}

/// Write one TSX tree branch.
fn emit_tsx_section(generator: &mut Generator, index: usize) {
    emit!(
        generator,
        "    <Section key={{items[{index}].id}} active={{items[{index}] satisfies Item}}>\n"
    );
    emit!(
        generator,
        "        {{items[{index}].children.map((child) => <Item value={{child.value ?? fallback}} />)}}\n"
    );
    generator.emit("    </Section>\n");
}
