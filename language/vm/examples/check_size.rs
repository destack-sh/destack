use destack_vm::{
    Continuation, Frame, HeapReference, Interpreter, Isolate, Module, RawPointer, StackPointer,
    Value,
};

fn main() {
    println!("Module: {} bytes", std::mem::size_of::<Module>());
    println!("Isolate: {} bytes", std::mem::size_of::<Isolate>());
    println!("Interpreter: {} bytes", std::mem::size_of::<Interpreter>());
    println!(
        "Continuation: {} bytes",
        std::mem::size_of::<Continuation>()
    );
    println!("Frame: {} bytes", std::mem::size_of::<Frame>());

    println!("Value: {} bytes", std::mem::size_of::<Value>());
    println!(
        "HeapReference: {} bytes",
        std::mem::size_of::<HeapReference>()
    );
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
