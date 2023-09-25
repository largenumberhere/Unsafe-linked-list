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
    /// Create and validate a layout
    fn layout(&self) -> Layout{
        Layout::from_size_align(size_of::<LLNode<T>>(), align_of::<LLNode<T>>()).expect("BOOMMM!")
    }

    /// Create a LinkedList with null nodes
    pub fn new() -> LinkedList<T> {
        LinkedList {
            head: null_mut(),
            tail: null_mut()
        }
    }

    /// Insert an item to the back of the linked list
    pub fn push_back(&mut self, value: T) {
        // Tell rust to not drop value
        let value = ManuallyDrop::new(value);

        let layout = self.layout();

        // Allocate manually
        let allocation = unsafe{
            assert_ne!(layout.size(), 0);
            /**
            Safety:
                Layout must not be zero,
                May return a null pointer,
                The given memory is "guaranteed to be initialized"
            **/
            let allocation = alloc_zeroed(layout) as *mut LLNode<T>;
            assert!(!allocation.is_null());

            //Write T to the node.
            /**
            Safety:
                ManuallyDrop<T> is guaranteed by design to have the same memory layout as T,
                so writing a ManuallyDrop<T> should be equivalent to writing a T
            **/
            let value_ptr: *mut ManuallyDrop<T> = addr_of_mut!((*allocation).value) as *mut ManuallyDrop<T>;

            /**
            Safety:
                - value_ptr is aligned because `self.layout()` used to create the allocation is aligned
                - the created layout is valid for writes because we just allocated initialized memory and ensured it was valid
            **/
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
                assert!(!last_node.is_null(), "This should never be null! The method should have returned early if self.head was null");
                loop {
                    /**
                    Safety:
                        - On previous iteration we checked that last_node was not null.
                        Before first iteration we asserted that last_node is not null.
                        - We known that last_node is aligned because we allocated it with an aligned layout
                        - We know that either next is a valid pointer to a valid field with valid layout *or* null_mut because all nodes default to this.
                    **/
                    if (*last_node).next.is_null() {
                        break;
                    }

                    /**
                    Safety:
                        Loop returns early if last_node is null,
                        We know that last_node is otherwise valid for reasons stated above
                    **/
                    last_node =(*last_node).next;
                }

                last_node
            };

            // Add the new node to the end of the last node
            /**
            Safety:
                - We know that last_node is not null from last iteration of loop or assertion above.
                - We know that last_node is aligned as stated above
                - We know that next is null and writable because all next fields on a node are either `null_mut()` or point to a initialized node with valid layout
             **/
            (*last_node).next = allocation;
            self.tail = (*last_node).next;
        }
    }

    pub fn first(&self) -> Option<&T> {
        /**
        Safety:
            - We know that head can be only one of 2 states, `null_mut()` as default *or* a pointer to a initialized aligned node as created in `push_back`
            - The data self points to cannot be removed without dropping the entire object, so we know that a reference to self will always be valid over this object's lifetime.
            - We provide only a non-mutable reference so with this and above, we can guarantee that there can only be non-mutable references to the underlying data
        **/
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
        /**
        Safety:
            - We know that head can be only one of 2 states, `null_mut()` as default *or* a pointer to a initialized aligned node as created in `push_back`
            - The data self points to cannot be removed without dropping the entire object, so we know that a reference to self will always be valid over this object's lifetime.
            - We provide only a non-mutable reference so with this and above, we can guarantee that there can only be non-mutable references to the underlying data
        **/
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

        if self.head.is_null(){
            return;
        }

        let layout = self.layout();
        // Dealocate nodes until there are none left
        loop {
            let mut last = self.head;
            let mut node = self.head;
            unsafe {

                // Traverse nodes until the last 2 are found
                /**
                Safety:
                    - We know that node is not null because we guarded against that earlier
                    - We know the node will otherwise be an aligned region of valid memory because we correctly allocated it earlier
                    - We know that node.next can only be `null_mut()` or valid node
                 **/
                loop{
                    if (*node).next.is_null(){
                        break;
                    }
                    last = node;
                    node = (*node).next;
                }

                // Call T's destructor
                /**
                Safety:
                    - We known the memory is valid for writing and reading because we allocated it earlier.
                    - We know the memory is aligned because self.layout specifies an alignment
                    - We know it's not null because we stop just before a null node above
                    - We know node is valid for dropping because we have not dropped it before and do not provide mutable access to it
                    - Node is not being accessed while it is being dropped because the destructor must only be called once and there is no reference allowed to self while or after drop
                    - Node's drop only drops T, it value is not used after drop is called
                 **/
                node.drop_in_place();

                // node's drop method only frees T, not the region of memory the node occupies, so we must manually deallocate
                // Deallocate the node and remove the pointer to it (or its next pointer if it is the last)

                /**
                Safety:
                    - We know from iteration above that node points to a valid, aligned node.
                    - We know that last.next is the only valid pointer/reference to last, because we only create one in this way and you cannot hold references to a dropped value.
                 **/
                (*last).next = null_mut();
                if node == self.head{
                    /**
                    Safety:
                        - We know we allocated this memory earlier with the same allocator
                        - We know from the guard clause above that self.head is not null and points to a node
                        - We know that layout is a valid, non-zero layout that is the same as the one used to allocate earlier
                     **/
                    dealloc(self.head as *mut u8, layout);
                    self.head = null_mut();
                    return;
                }

                /**
                Safety:
                    - We know that node is not null as per iteration above and points to a node
                    - We know that node was created with the same layout - self.layout
                    - We know that node is created with the same allocator
                 **/
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