use destack_machine::memory::{HeapHandle, RawPointer, StackPointer, Value};

fn main() {
    println!("Value: {} bytes", std::mem::size_of::<Value>());
    println!("HeapHandle: {} bytes", std::mem::size_of::<HeapHandle>());
    println!("RawPointer: {} bytes", std::mem::size_of::<RawPointer>());
    println!(
        "StackPointer: {} bytes",
        std::mem::size_of::<StackPointer>()
    );
    println!(
        "Option<Value>: {} bytes",
        std::mem::size_of::<Option<Value>>()
    );
}
