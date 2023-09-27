extern crate core;

//Unfinished - need to learn to use alloc first!
//mod linked_list;
//mod linked_list2;
//mod ll3;
mod ll4;

/// test for memory bugs with: `cargo miri run`.
///  Does not use the integrated rust test suite
///  because it makes memory bug testing much more difficult.
fn main() {

    ll4::testing::add();
    ll4::testing::add2();
    //println!("strings");
    ll4::testing::add_strings();
    ll4::testing::first();
    ll4::testing::get();
    ll4::testing::create_destroy();
    ll4::testing::into_iter();
    ll4::testing::into_iter_partial_use();
}
