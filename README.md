# Unsafe linked list
A linked list implementation in unsafe rust with super low level memory control.

It is written in primarily unsafe rust using the global allocator's `alloc_zeroed` and `dealloc`. 
It explores some of their uses.
Features notable mentions of `ManuallyDrop<T>`, `core::ptr::mut_ptr::drop_in_place`, 
writing to raw memory, reading to raw memory, many raw pointers, dereferencing pointers, null pointers, among many other naughty memory tricks
that you should never want or need in rust!


### Build
- Clone the repository 
- Make sure you have [miri](https://github.com/rust-lang/miri) installed. You can add it with `rustup +nightly component add miri`
- Use `cargo miri run` inside of the root folder to run the tests with miri's memory checking
