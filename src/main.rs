extern crate core;

//Unfinished - need to learn to use alloc first!
//mod linked_list;
//mod linked_list2;
//mod ll3;
mod ll4;

/// test for memory bugs with: `cargo miri run`
fn main() {

    ll4::testing::add();
    ll4::testing::add2();
    println!("strings");
    ll4::testing::add_strings();
    ll4::testing::first();
    ll4::testing::get();
}
