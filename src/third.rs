/// list1 = A -> B -> C -> D
/// list2 = tail(list1) = B -> C -> D
/// list3 = push(list2, X) = X -> B -> C -> D
///
/// list1 -> A ---v
/// list2 ------> B -> C -> D
/// list3 -> X ---^
///
/// Rust doesn't have anything like the garbage collectors these languages have. They have tracing
/// GC, which will dig through all the memory that's sitting around at runtime and figure out
/// what's garbage automatically. Instead, all Rust has today is reference counting. Reference
/// counting is basically a poor-man's GC. For many workloads, it has significantly less throughput
/// than a tracing collector, and it completely falls over if you manage to build cycles.
/// Thankfully, for our usecase we'll never run into cycles (feel free to try to prove this to
/// yourself -- I sure won't).
///
///
/// So how do we do reference counted garbage collection? Rc! Rc is just like Box, but we can
/// duplicate it, and its memory will only be freed when all the Rc's derived from are dropped.
/// Unforuntately, this flexibility comes at a serious cost: we can only Deref an Rc. No DerefMut
/// or DerefMove. This means we can't ever really get data out of one of our lists, nor can we
/// mutate them.
///

/// We need reference counting now
use std::rc::Rc;

pub struct List<T> {
    head: Link<T>,
}

type Link<T> = Option<Rc<Node<T>>>;

struct Node<T> {
    elem: T,
    next: Link<T>,
}

pub struct Iter<'a, T: 'a> {
    next: Option<&'a Node<T>>,
}

impl<T> List<T> {
    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter {
            next: self.head.as_ref().map(|node| &**node),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.next.as_ref().map(|node| &**node);
            &node.elem
        })
    }
}

impl<T> List<T> {
    pub fn new() -> Self {
        List { head: None }
    }

    /// append() takes a list and an element, and returns a List
    ///
    /// Like the mutable list case, we want to make a new node, that has the old list as its next
    /// value. The only novel thing is how to get that next value, because we're not allowed to
    /// mutate anything.
    ///
    /// The answer to our prayers is the Clone trait. Clone is implemented by almost every type,
    /// and provides a generic way to get "another one like this one" that is logically disjoint
    /// given only a shared reference. It's like a copy constructor in C++, but it's never
    /// implicitly invoked.
    ///
    /// Rc in particular uses Clone as the way to increment the reference count. So rather than
    /// moving a Box to be in the sublist, we just clone the head of the old list. We don't even
    /// need to match on the head, because Option exposes a Clone implementation that does exactly
    /// the thing we want.
    pub fn append(&self, elem: T) -> List<T> {
        List {
            head: Some(Rc::new(Node {
                elem: elem,
                next: self.head.clone(),
            })),
        }
        //tOdod
    }

    /// tail is the logical inverse of this operation. It takes a list and removes the whole list
    /// with the first element removed. All that is is cloning the second element in the list (if
    /// it exists).
    pub fn tail(&self) -> List<T> {
        List {
            /// and_then
            head: self.head.as_ref().and_then(|node| node.next.clone()),
        }
    }

    /// head() returns a reference to the first element.
    ///
    /// That's just peek from the mutable list.
    pub fn head(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.elem)
    }
}

/// recursive destructor
///
/// ```ignore
/// impl<T> Drop for List<T> {
///     fn drop(&mut self) {
///         let mut cur_link = self.head.take();
///         while let Some(mut boxed_node) = cur_link {
///             // mutating the Node inside the Box
///             cur_link = boxed_node.next.take();
///         }
///     }
/// }
/// ```
impl<T> Drop for List<T> {
    /// a recursive deconstructor that works in O(n)
    ///
    /// The first way is that we can keep grabbing the tail of the list and dropping the previous
    /// one to decrement its count. This will prevent the old list from recursively dropping the
    /// rest of the list because we hold an outstanding reference to it. This has the unfortunate
    /// problem that we traverse the entire list whenever we drop it. In particular this means
    /// building a list of length n in place takes O(n2) as we traverse a lists of length n-1, n-2,
    /// .., 1 to guard against overflow.
    fn drop(&mut self) {
        // Steal the list's head
        let mut cur_list = self.head.take();
        while let Some(node) = cur_list {
            // Clone the current node's next node.
            cur_list = node.next.clone();
            // Node dropped here. If the old node had
            // refcount 1, then it will be dropped and freed, but it won't
            // be able to fully recurse and drop its child, because we
            // hold another Rc to it.
        }
    }
}
/// a recursive deconstructor that works in amortized O(1)
///
/// The second way is if we could identify that we're the last list that knows about this node,
/// we could in principle actually move the Node out of the Rc. Then we could also know when to
/// stop: whenver we can't hoist out the Node. For reference, the unstable function is called
/// try_unwrap.
///impl<T> Drop for List<T> {
/// fn drop(&mut self) {
/// let mut head = self.head.take();
/// while let Some(node) = head {
///     if let Ok(mut node) = Rc::try_unwrap(node) {
///         head = node.next.take();
///     } else {
///         break;
///     }
/// }
/// }
/// }

#[cfg(test)]
mod test {
    use super::List;

    #[test]
    fn basics() {
        let list = List::new();
        assert_eq!(list.head(), None);

        let list = list.append(1).append(2).append(3);
        assert_eq!(list.head(), Some(&3));

        let list = list.tail();
        assert_eq!(list.head(), Some(&2));

        let list = list.tail();
        assert_eq!(list.head(), Some(&1));

        let list = list.tail();
        assert_eq!(list.head(), None);

        let list = list.tail();
        assert_eq!(list.head(), None);
    }

    #[test]
    fn iter() {
        let list = List::new().append(1).append(2).append(3);

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&1));
    }
}
