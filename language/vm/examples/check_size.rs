use destack_heap::HeapReference;
use destack_vm::{Cell, Continuation, Frame, Machine, Program, StackPointer};

fn main() {
    println!("Program: {} bytes", std::mem::size_of::<Program>());
    println!("Machine: {} bytes", std::mem::size_of::<Machine>());
    println!(
        "Continuation: {} bytes",
        std::mem::size_of::<Continuation>()
    );
    println!("Frame: {} bytes", std::mem::size_of::<Frame>());

    println!("Cell: {} bytes", std::mem::size_of::<Cell>());
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
        "Option<Cell>: {} bytes",
        std::mem::size_of::<Option<Cell>>()
    );
}
