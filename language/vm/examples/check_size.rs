use destack_vm::{
    Continuation, Frame, HeapReference, Interpreter, Isolate, Program, StackPointer, Word,
};

fn main() {
    println!("Program: {} bytes", std::mem::size_of::<Program>());
    println!("Isolate: {} bytes", std::mem::size_of::<Isolate>());
    println!("Interpreter: {} bytes", std::mem::size_of::<Interpreter>());
    println!(
        "Continuation: {} bytes",
        std::mem::size_of::<Continuation>()
    );
    println!("Frame: {} bytes", std::mem::size_of::<Frame>());

    println!("Word: {} bytes", std::mem::size_of::<Word>());
    println!(
        "HeapReference: {} bytes",
        std::mem::size_of::<HeapReference>()
    );
    println!("address: {} bytes", std::mem::size_of::<usize>());
    println!(
        "StackPointer: {} bytes",
        std::mem::size_of::<StackPointer>()
    );
    println!(
        "Option<Word>: {} bytes",
        std::mem::size_of::<Option<Word>>()
    );
}
