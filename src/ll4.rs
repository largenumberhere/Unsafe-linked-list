use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::env::current_exe;
use std::fmt::{Debug, Display, Formatter};
use std::mem::{align_of, ManuallyDrop, size_of};
use std::ops::Deref;
use std::ptr::{addr_of_mut, NonNull, null_mut};
use std::thread::current;

struct LinkedList<T> {
    head: *mut LLNode<T>,
    tail: *mut LLNode<T>
}

#[derive(Clone)]
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
            /*
            Safety:
                Layout must not be zero,
                May return a null pointer,
                The given memory is "guaranteed to be initialized"
            */
            let allocation = alloc_zeroed(layout) as *mut LLNode<T>;
            assert!(!allocation.is_null());

            //Write T to the node.
            /*
            Safety:
                ManuallyDrop<T> is guaranteed by design to have the same memory layout as T,
                so writing a ManuallyDrop<T> should be equivalent to writing a T
            */
            let value_ptr: *mut ManuallyDrop<T> = addr_of_mut!((*allocation).value) as *mut ManuallyDrop<T>;

            /*
            Safety:
                - value_ptr is aligned because `self.layout()` used to create the allocation is aligned
                - the created layout is valid for writes because we just allocated initialized memory and ensured it was valid
            */
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
                    /*
                    Safety:
                        - On previous iteration we checked that last_node was not null.
                        Before first iteration we asserted that last_node is not null.
                        - We known that last_node is aligned because we allocated it with an aligned layout
                        - We know that either next is a valid pointer to a valid field with valid layout *or* null_mut because all nodes default to this.
                    */
                    if (*last_node).next.is_null() {
                        break;
                    }

                    /*
                    Safety:
                        Loop returns early if last_node is null,
                        We know that last_node is otherwise valid for reasons stated above
                    */
                    last_node =(*last_node).next;
                }

                last_node
            };

            // Add the new node to the end of the last node
            /*
            Safety:
                - We know that last_node is not null from last iteration of loop or assertion above.
                - We know that last_node is aligned as stated above
                - We know that next is null and writable because all next fields on a node are either `null_mut()` or point to a initialized node with valid layout
             */
            (*last_node).next = allocation;
            self.tail = (*last_node).next;
        }
    }

    pub fn first(&self) -> Option<&T> {
        /*
        Safety:
            - We know that head can be only one of 2 states, `null_mut()` as default *or* a pointer to a initialized aligned node as created in `push_back`
            - The data self points to cannot be removed without dropping the entire object, so we know that a reference to self will always be valid over this object's lifetime.
            - We provide only a non-mutable reference so with this and above, we can guarantee that there can only be non-mutable references to the underlying data
        */
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
        /*
        Safety:
            - We know that head can be only one of 2 states, `null_mut()` as default *or* a pointer to a initialized aligned node as created in `push_back`
            - The data self points to cannot be removed without dropping the entire object, so we know that a reference to self will always be valid over this object's lifetime.
            - We provide only a non-mutable reference so with this and above, we can guarantee that there can only be non-mutable references to the underlying data
        */
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

    /// Get pointer to the first item and remove it from the linked list if it is not null.
    /// you must manually deallocate the memory at this location after use with the layout given by this structs layout method.
    /// Using the wrong layout can cause undefined behaviour.
    /// Not deallocating will cause memory leaks.
    /// All pointers to the former head value will become invalidated.
    unsafe fn yank_first(&mut self) -> Option<*mut LLNode<T>> {
        if self.head.is_null(){
            return None;
        }
        /*
        Safety:
            - we know self.head is not null on previous line
            - we know that self.head is otherwise aligned and valid because aligned and allocated
              or null_mut are the only 2 pointer states we create for head
        */
        let next = (*self.head).next;
        let head = self.head;

        self.head = next;
        return Some(head);
    }

    /// Drop all the nodes contained in this Linked list without moving ownership.
    /// Invalidates all pointers/ references to nodes that were in this linkedlist.
    ///
    unsafe fn drop_inner(&mut self){
        let layout = self.layout();

        let mut current = self.head;
        // Deallocate nodes front to back, until there are none left
        loop {
            unsafe {
                if current.is_null() {
                    return;
                }
                /*
                Safety:
                    - We know current is not null,
                    - We know current is aligned because we allocated it with valid layout
                    - We know that current can only be null_mut() (which is caught by the the guard clause before this) or a
                        valid pointer because it is initialized to null and the only state we set it to is pointing to a valid node.
                 */
                let next_ptr = (*current).next;

                /*
                Safety:
                    - We known the memory is valid for writing and reading because we allocated it earlier and haven't yet dealocated it
                    - We know the memory is aligned because self.layout specifies an checked alignment
                    - We know it's not null because we stop just before a null node above
                    - We know node is valid for dropping because we have not dropped it before and do not provide mutable access to it
                    - Node is not being accessed while it is being dropped because the destructor must only be called once and there is no reference allowed to self while or after drop
                    - Node's drop only drops T, it value is not used after drop is called
                 */
                current.drop_in_place();

                /*
                Safety:
                    - We know that current is not null as checked above and the only value we set is a node
                    - We know that current was created with the same valid layout - self.layout
                    - We know that current is created with the same allocator
                 */
                dealloc(current as *mut u8, layout);

                current = next_ptr;
            }
        }
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        /*
        Safety:
            - All references to LinkedList or its nodes are now unable to be held, doing so would violate lifetime rules.
            - No raw pointers are made externally available to the nodes and we do not use them after calling drop
        */
        unsafe{ self.drop_inner()};
    }
}

struct IntoIter<T>
{
    linked_list : ManuallyDrop<LinkedList<T>>,
    layout: Layout
}

/// basically a crude reinvention of Box<LLNode<T>>
struct LinkedListItem<T>
{
    heap_allocated_node : *mut LLNode<T>,
    layout: Layout,
}

impl<T> Drop for LinkedListItem<T> {
    fn drop(&mut self) {
        unsafe {
            /*
            Safety:
                - We allocated this memory earlier so we know it is valid for reads and writes
                - We used a validated layout so we know it's aligned
                - We did not create a null LinkedListItem, so we know it can't be null
                - We assume the library consumer is use safe code,
                   and thus holds no pointer to the values contained in T
                - We do not allow references or pointers to the nodes after drop_in_place is called,
                   so no pointers can become invalidated.
            */
            self.heap_allocated_node.drop_in_place();

            /*
            Safety:
                - We know this memory is allocated with this allocator because we did so earlier.
                - We know this is the same valid layout we used to allocate this region of memory
            */
            dealloc(self.heap_allocated_node as *mut u8, self.layout);
        }
    }
}

impl<T> LinkedListItem<T> {
    fn value(&self) -> &T {
        /*
        Safety:
            - We know self.heap_allocated_node is not null, because the next method which crates
               this only does so if `yank_first` succeeds. It does not construct this struct if the node is null.
            - We know self.heap_allocated_node uses a valid layout because all non-null nodes are allocated with a validated layout.
        */
        let val = & unsafe{ &*self.heap_allocated_node }.value ;
        val
    }
}

impl<T> Deref for LinkedListItem<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value()
    }

}

impl<T> Debug for LinkedListItem<T> where T: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.value(), f)
    }
}

impl<T> Display for LinkedListItem<T>  where T: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
       Display::fmt(self.value(),f)
    }
}




impl<T> Iterator for IntoIter<T>
{
    type Item = LinkedListItem<T>;
    fn next(&mut self) -> Option<Self::Item>{
        // Remove first node
        /*
        Safety:
            - We immediately wrap the data in a struct that deallocates with the correct layout on drop,
               so no memory leaks should occur.
            - No references are allowed to any nodes after into_iterator is called, so this will not invalidate any refernces.
            - No public facing are given so they cannot be invalidated.
        */
        let node = unsafe {self.linked_list.yank_first()}?;

        // Place it in a struct that only allows access to the item and deallocates on drop
        let item = LinkedListItem {
            heap_allocated_node: node,
            layout: self.layout
        };

        Some(item)
    }
}


impl<T> IntoIterator for LinkedList<T>
{
    type Item = LinkedListItem<T>;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        // Do not drop nodes until we say
        let nodes = ManuallyDrop::new(self);
        IntoIter {
            layout: nodes.layout(),
            linked_list: nodes,
        }
    }
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        /*
        Safety:
            - No references are allowed to a struct after drop is called to is, so the user cannot hold stale references.
            - No public facing pointer access is available, so they cannot be stale pointers.
        */
        unsafe{ self.linked_list.drop_inner();}
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

    pub fn create_destroy() {
        let ll: LinkedList<String> = LinkedList::new();
        std::mem::drop(ll);
    }

    pub fn into_iter() {
        let mut ll = LinkedList::new();
        ll.push_back("Hello".to_string());
        ll.push_back("World".to_string());

        let iterator = ll.into_iter();
        for string in iterator{
            println!("{}", *string);
        }
    }

    pub fn into_iter_partial_use() {
        let mut ll = LinkedList::new();
        for i in 0..128{
            ll.push_back(format!("hello! {}", i));
        }

        let mut iterator = ll.into_iter();
        for i in 0..64 {
            assert_eq!(iterator.next().as_deref(), Some(format!("hello! {}", i)).as_ref());
        }
    }





}

// ptr::read(p), copies data out of managed memory. If p implements drop, so will the returned value
// ptr::write(T) copies data into managed memory. If T implements drop, it will not be called