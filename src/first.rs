use std::mem;

/// functional programming style:
/// ```ignore
/// List a = Empty | Elem a (List a)
/// ```
/// This means "A List is either Empty or an Element followed by a List" in functional programming.
/// However, in Rust we don't want to have a layout like functional programming. The previous
/// struct of `List` will give us a layout like:
/// ```ignore
/// [Elem A, ptr] -> (Elem B, ptr) -> (Elem C, ptr) -> (Empty *junk*)<Paste>
/// [] = Stack
/// () = Heap
/// ```
/// In this case a split off C means:
/// ```ignore
/// [Elem A, ptr] -> (Elem B, ptr) -> (Empty *junk*)
/// [Elem C, ptr] -> (Empty *junk*)
/// ```
/// On the other hand, a Rust implementation would expect the layout and split to be like:
/// ```ignore
/// layout 2:
/// [ptr] -> (Elem A, ptr) -> (Elem B, ptr) -> (Elem C, *null*)
///
/// split off C:
/// [ptr] -> (Elem A, ptr) -> (Elem B, *null*)
/// [ptr] -> (Elem C, *null*)
/// ```
/// This results in a implementation:
/// ```ignore
/// struct Node {
///     elem: i32,
///     next: List,
/// }
///
/// pub enum List {
///     Empty,
///     More(Box<Node>),
/// }
/// ```
/// , which fulfills the following:
/// - Tail of a list never allocates extra junk: check!
/// - `enum` is in delicious null-pointer-optimized form: check!
/// - All elements are uniformly allocated: check!
///
/// However, there is one last problem we need to fix to implement our `List`. We want to make our
/// List public, but the Node private --- while a public `enum` will make everything public.
///
/// List is a public struct
pub struct List {
    head: Link,
}

/// Link is a private enum as we want to hide the implementation details
enum Link {
    Empty,
    /// Because we don't know the how many elements are there hence how much memory to allocate for
    /// the `Node` (`List`), we need a `box` so that Rust understand how big `Node` needs to be.
    More(Box<Node>),
}

/// Node is a private struct
struct Node {
    elem: i32,
    next: Link,
}

/// Notes on impl:
/// - `Self`is an alias for "that type I wrote at top next to impl". Great for not repeating yourself!
/// - We create an instance of a struct in much the same way we declare it, except instead of providing the types of its fields, we initialize them with values.
/// - We refer to variants of an enum using ::, which is the namespacing operator.
/// - The last expression of a function is implicitly returned. This makes simple functions a little neater. You can still use return to return early like other C-like languages.
///
/// Notes on ownership (https://cglab.ca/~abeinges/blah/too-many-lists/book/first-ownership.html):
/// - `self`      - Value
/// - `&mut self` - mutable reference. The only thing you can't do with an &mut is move the value
/// out with no replacement.
/// - `&self`     - shared reference
impl List {
    /// new() will take no parameter and construct an empty list for us
    pub fn new() -> Self {
        List { head: Link::Empty }
    }

    /// push() will take a list and an element, and return us a list.
    ///
    /// Note that in the implementation, we cannot directly add self.head to be the next element in
    /// the new node, This is the only thing we can't do with an `&mut` --- moveing the value out
    /// with no replacement.
    pub fn push(&mut self, elem: i32) {
        let new_node = Box::new(Node {
            elem: elem,
            next: mem::replace(&mut self.head, Link::Empty),
        });

        self.head = Link::More(new_node);
    }

    /// pop() will
    ///
    /// ```ignore
    /// pub fn pop(&mut self) -> Option<i32> {
    ///     let result;
    ///     match self.head {
    ///         Link::Empty => {
    ///             result = None;
    ///
    ///         }
    ///         Link::More(ref node) => {
    ///             result = Some(node.elem);
    ///             self.head = node.next;
    ///         }
    ///     };
    ///     result
    /// }
    ///
    /// ```
    /// ```ignore
    /// pub fn pop(&mut self) -> Option<i32> {
    ///     let result;
    ///     match mem::replace(&mut self.head, Link::Empty) {
    ///         Link::Empty => {
    ///             result = None;
    ///         }
    ///         Link::More(node) => {
    ///             result = Some(node.elem);
    ///             self.head = node.next;
    ///         }
    ///     };
    ///     result
    /// }
    /// ```
    /// Box is actually really special in Rust, because it's sufficiently built into the language
    /// that the compiler lets you do some stuff that nothing else can do. We actually have been
    /// doing one such thing this whole time: DerefMove. Whenever you have a pointer type you can
    /// derefence it with * or . to get at its contents. Usually you can get a Deref or maybe even
    /// a DerefMut, corresponding to a shared or mutable reference respectively.
    ///
    /// However because Box totally owns its contents, you can actually move out of a dereference.
    /// This is total magic, because there's no way for any other type to opt into this. There's
    /// tons of other cool tricks the compiler knows how to do with Box because it just is Box, but
    /// they were all feature-gated at 1.0 pending further design. Ideally Box will be totally user
    /// definable in the future.
    ///
    pub fn pop(&mut self) -> Option<i32> {
        match mem::replace(&mut self.head, Link::Empty) {
            Link::Empty => None,
            Link::More(boxed_node) => {
                let node = *boxed_node;
                self.head = node.next;
                Some(node.elem)
            }
        }
    }
}

/// ```ignore
/// impl Drop for List {
///     fn drop(&mut self) {
///         // NOTE: you can't actually explicitly call `drop` in real Rust code;
///         // we're pretending to be the compiler!
///         list.head.drop(); // tail recursive - good!
///     }
/// }
///
/// impl Drop for Link {
///     fn drop(&mut self) {
///         match list.head {
///             Link::Empty => {} // Done!
///             Link::More(ref mut boxed_node) => {
///                 boxed_node.drop(); // tail recursive - good!
///             }
///         }
///     }
/// }
///
/// impl Drop for Box<Node> {
///     fn drop(&mut self) {
///         self.ptr.drop(); // uh oh, not tail recursive!
///         deallocate(self.ptr);
///     }
/// }
///
/// impl Drop for Node {
///     fn drop(&mut self) {
///         self.next.drop();
///     }
/// }
///
/// ```
impl Drop for List {
    /// Basically, "when you go out of scope, I'll give you a second to clean up your affairs".
    fn drop(&mut self) {
        let mut cur_link = mem::replace(&mut self.head, Link::Empty);
        // `while let` == "do this thing until this pattern doesn't match"
        while let Link::More(mut boxed_node) = cur_link {
            cur_link = mem::replace(&mut boxed_node.next, Link::Empty);
            // boxed_node goes out of scope and gets dropped here;
            // but its Node's `next` field has been set to Link::Empty
            // so no unbounded recursion occurs.
        }
    }
}

#[cfg(test)]
mod test {
    use super::List;

    #[test]
    fn basics() {
        let mut list = List::new();

        // Check empty list behaves right
        assert_eq!(list.pop(), None);

        // Populate list
        list.push(1);
        list.push(2);
        list.push(3);

        // Check normal removal
        assert_eq!(list.pop(), Some(3));
        assert_eq!(list.pop(), Some(2));

        // Push some more just to make sure nothing's corrupted
        list.push(4);
        list.push(5);

        // Check normal removal
        assert_eq!(list.pop(), Some(5));
        assert_eq!(list.pop(), Some(4));

        // Check exhaustion
        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), None);
    }
}
