use destack_vm::{
    Continuation, Executable, Frame, Interpreter, Isolate, ManagedReference, RawPointer,
    StackPointer, Value,
};

fn main() {
    println!("Executable: {} bytes", std::mem::size_of::<Executable>());
    println!("Isolate: {} bytes", std::mem::size_of::<Isolate>());
    println!("Interpreter: {} bytes", std::mem::size_of::<Interpreter>());
    println!(
        "Continuation: {} bytes",
        std::mem::size_of::<Continuation>()
    );
    println!("Frame: {} bytes", std::mem::size_of::<Frame>());

    println!("Value: {} bytes", std::mem::size_of::<Value>());
    println!(
        "ManagedReference: {} bytes",
        std::mem::size_of::<ManagedReference>()
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
