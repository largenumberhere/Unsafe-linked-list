use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::mem::{align_of, ManuallyDrop, size_of};
use std::ptr::{addr_of_mut, null_mut};

struct LinkedList<T> {
    head: *mut LLNode<T>,
    tail: *mut LLNode<T>
}

struct LLNode<T> {
    //last: *mut LLNode<T>,
    next: *mut LLNode<T>,
    value: T 
}


impl<T> LinkedList<T> {
    fn layout(&self) -> Layout{
        Layout::from_size_align(size_of::<LLNode<T>>(), align_of::<LLNode<T>>()).expect("BOOMMM!")
    }

    pub fn new() -> LinkedList<T> {
        LinkedList {
            head: null_mut(),
            tail: null_mut()
        }
    }

    pub fn push_back(&mut self, value: T) {
        // Tell rust to not drop value
        let value = ManuallyDrop::new(value);

        let layout = self.layout();

        // Allocate manually
        let allocation = unsafe{
            let allocation = alloc_zeroed(layout) as *mut LLNode<T>;
            assert!(!allocation.is_null());

            //Write T to the node
            let value_ptr: *mut ManuallyDrop<T> = addr_of_mut!((*allocation).value) as *mut ManuallyDrop<T>;
            value_ptr.write(value);

            allocation
        };

        // Insert the node and return
        if self.head.is_null() {
            self.head = allocation;
            self.tail = self.head;
            return;
        }

        unsafe {
            // Find the last node
            let last_node = {
                let mut last_node: *mut LLNode<T> = self.head;
                assert!(!last_node.is_null(), "This should never be null! The method should have returned early if self.head is null");
                loop {
                    if (*last_node).next.is_null() {
                        break;
                    }
                    last_node =(*last_node).next;
                }

                last_node
            };

            // Add the new node to the end of the last node
            (*last_node).next = allocation;
            self.tail = (*last_node).next;
        }
    }

    pub fn first(&self) -> Option<&T> {
        let head = unsafe{ self.head.as_ref()};
        let reference = match head {
            Some(v) => {
                &v.value
            }
            None=> {
                return None;
            }
        };

        Some(reference)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        // Traverse nodes until next is null or node_option is found
        let mut node_option = unsafe{ self.head.as_ref() };
        for _ in 0..index{
            node_option = match node_option {
                Some(v) => {
                    unsafe{ v.next.as_ref() }
                }

                None => {
                    return None;
                }
            }
        }

        // Return None if the i-th node happened to be null
        let node = node_option?;

        let value = &node.value;
        Some(value)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {

        let layout = self.layout();

        // Dealocate nodes until there are none left
        loop {
            let mut last = self.head;
            let mut node = self.head;
            unsafe {

                // Traverse nodes until the last 2 are found
                loop{
                    if (*node).next.is_null(){
                        break;
                    }
                    last = node;
                    node = (*node).next;
                }

                // Call T's destructor
                node.drop_in_place();

                // Deallocate the node and remove the pointer to it (or its next pointer if it is the last)
                (*last).next = null_mut();
                if node == self.head{
                    dealloc(self.head as *mut u8, layout);
                    self.head = null_mut();
                    return;
                }

                dealloc(node as *mut u8, layout);
            }
        }

    }
}


//#[cfg(test)]
pub mod testing {
    use crate::ll4::LinkedList;

    //#[test]
    pub fn add() {
        let mut ll = LinkedList::new();
        ll.push_back(2);
        let head = unsafe{  ll.head.read() };
        assert_eq!(head.value, 2);

        ll.push_back(3);
        //println!("5");
        let head = unsafe{ ll.head.read() };
        //println!("{:p}", head.next);
        let next = unsafe{ head.next.read() }.value;
        let head = unsafe{ ll.head.read() };

        //println!("6");
        assert_eq!(next, 3);
        assert_eq!(head.value, 2);
        std::mem::drop(ll);
    }

    pub fn add2() {
        let mut ll = LinkedList::new();
        for i in 0..100 {
            ll.push_back(i);
        }
    }

    pub fn add_strings() {
        for _ in 0..2 {
            let mut ll = LinkedList::new();
            for i in 0..16 {
                let string = format!("item-{}", i);
                //println!("{:p}", string.as_ptr());
                ll.push_back(string);
            }

            std::mem::drop(ll);
        }
    }

    pub fn first() {
        let mut ll = LinkedList::new();
        for i in 0..3 {
            ll.push_back(format!("Helloooo!!! {}", i));
            assert_eq!(ll.first(), Some("Helloooo!!! 0".to_string()).as_ref());
            ll.push_back(format!("Helloooo againn!!! {}", i));
        }

        assert_eq!(ll.first(), Some("Helloooo!!! 0".to_string()).as_ref());
    }

    pub fn get() {
        let mut ll = LinkedList::new();
        for i in 0 ..32 {
            let string = format!("node {}", i);
            ll.push_back(string);

            //Make sure all nodes inserted return Some
            for i2 in 0..=i {
                let string = format!("node {}", i2);
                assert_eq!(ll.get(i2), Some(string).as_ref());
            }

            //Make sure nodes after max inserted return none
            for i3 in i+1..4 {
                assert_eq!(ll.get(i + i3), None);
            }
        }

    }



}

// ptr::read(p), copies data out of managed memory. If p implements drop, so will the returned value
// ptr::write(T) copies data into managed memory. If T implements drop, it will not be called