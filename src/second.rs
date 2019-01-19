//use std::mem;
//use crate::second::Iterator;

pub struct List<T> {
    head: Link<T>,
}

/// Type alias: a short way to implement Link.
///
/// A previous example is:
/// ```ignore
/// enum Link {
///     Empty,
///     More(Box<Node>),
/// }
/// ```
type Link<T> = Option<Box<Node<T>>>;

struct Node<T> {
    elem: T,
    next: Link<T>,
}

/// ```
/// pub trait Iterator {
///     type Item;
///     fn next(&mut self) -> Option<Self::Item>;
/// }
/// ```
///
///
/// IntoIter is a type just wrapper around List.
pub struct IntoIter<T>(List<T>);

// Iter is generic over *some* lifetime, it doesn't care
pub struct Iter<'a, T: 'a> {
    next: Option<&'a Node<T>>,
}

pub struct IterMut<'a, T: 'a> {
    next: Option<&'a mut Node<T>>,
}

impl<T> List<T> {
    pub fn into_iter(self) -> IntoIter<T> {
        return IntoIter(self);
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        return self.0.pop();
    }
}

impl<T> List<T> {
    /// Note that there nothing pointy in this method -- we don't need to change anything to make
    /// the *generic* work.
    ///
    /// Bask in the Glory that is `Self`, guardian of refactoring and copy-pasta coding. Also of
    /// interest, we don't write `List<T>` when we construct an instance of list. That part's
    /// inferred for us based on the fact that we're returning it from a function that expects a
    /// `List<T>`.
    pub fn new() -> Self {
        List { head: None }
    }

    /// Note: Because  `mem::replace(&mut option, None)` is such an incredibly common idiom that
    /// Option actually just went ahead and made it a method: `take`. Thus, before we have
    /// ```ignore
    /// next: mem::replace(&mut self.head, None),
    /// ```
    /// Now, we do
    /// ```ignore
    /// next: self.head.take(),
    /// ```
    pub fn push(&mut self, elem: T) {
        let new_node = Box::new(Node {
            elem: elem,
            next: self.head.take(),
        });

        self.head = Some(new_node);
    }

    /// Note:  `match option { None => None, Some(x) => Some(y) }` is such an incredibly common
    /// idiom that it was called  `map`. `map` takes a function to execute on `x` in the `Some(x)`
    /// to produce the `y` in `Some(y)`. We could write a proper fn and pass it to map, but we'd
    /// much rather write what to do inline.
    ///
    /// The way to do this is with a *closure*. Closures are anonymous functions with an extra
    /// super-power: they can refer to local variables outside the closure! This makes them super
    /// useful for doing all sorts of conditional logic.
    ///
    /// Below is the previous impl.
    /// ```ignore
    /// pub fn pop(&mut self) -> Option<i32> {
    ///     match mem::replace(&mut self.head, None) {
    ///         None => None,
    ///         Some(node) => {
    ///             let node = *node;
    ///             self.head = node.next;
    ///             Some(node.elem)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        self.head.take().map(|node| {
            let node = *node;
            self.head = node.next;
            node.elem
        })
    }

    /// Note that Rust won't allow the following implementation
    /// ```ignore
    /// pub fn peek(&self) -> Option<&T> {
    ///     self.head.map(|node| {
    ///         &node.elem
    ///     })
    /// }
    /// ```
    /// , becase map takes `self` by value, which would move the Option out of the thing it's in.
    /// Previously this was fine because we had just `take`n it out, but now we actually want to
    /// leave it where it was. The correct way to handle this is with the `as_ref` method on
    /// Option.
    ///
    /// Additionally, it demotes the Option to an Option to a reference to its internals. We could
    /// do this ourselves with an explicit match but ugh no. It does mean that we need to do an
    /// extra derefence to cut through the extra indirection, but thankfully the . operator handles
    /// that for us.
    pub fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.elem)
    }

    /// Mutable version of `peek()`
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.head.as_mut().map(|node| &mut node.elem)
    }
}

/// ```
/// impl Drop for List {
///     fn drop(&mut self) {
///         let mut cur_link = mem::replace(&mut self.head, None);
///         while let Some(mut boxed_node) = cur_link {
///             cur_link = mem::replace(&mut boxed_node.next, None);
///         }
///     }
/// }
/// ```
impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut cur_link = self.head.take();
        // `while let` == "do this thing until this pattern doesn't match"
        while let Some(mut boxed_node) = cur_link {
            cur_link = boxed_node.next.take();
            // boxed_node goes out of scope and gets dropped here;
            // but its Node's `next` field has been set to Link::Empty
            // so no unbounded recursion occurs.
        }
    }
}

// No lifetime here, List doesn't have any associated lifetimes
impl<T> List<T> {
    /// Note that this is syntax sugar for
    /// ```ignore
    /// impl<T> List<T> {
    ///     pub fn iter<'a>(&'a self) -> Iter<'a, T> {
    ///         Iter { next: self.head.as_ref().map(|node| &**node)
    ///     }
    /// }
    /// ```
    // We declare a fresh lifetime here for the *exact* borrow that
    // creates the iter. Now &self needs to be valid as long as the
    // Iter is around.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            next: self.head.as_ref().map(|node| &**node),
        }
    }
}

/// This is syntax sugar for:
/// ```ignore
/// impl<'a, T> Iterator for Iter<'a, T> {
///     type Item = &'a T;
///
///     fn next<'b>(&'b mut self) -> Option<&'a T> { /* stuff */ }
/// }
/// ```

// *Do* have a lifetime here, because Iter does have an associated lifetime
impl<'a, T> Iterator for Iter<'a, T> {
    // Need it here too, this is a type declaration
    type Item = &'a T;

    // None of this needs to change, handled by the above.
    // Self continues to be incredibly hype and amazing
    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.next.as_ref().map(|node| &**node);
            &node.elem
        })
    }
}

impl<T> List<T> {
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            next: self.head.as_mut().map(|node| &mut **node),
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_mut().map(|node| &mut **node);
            &mut node.elem
        })
    }
}

#[cfg(test)]
mod test {
    use super::List;

    #[test]
    fn basics() {
        let mut list = List::new();

        assert_eq!(list.pop(), None);

        list.push(1);
        list.push(2);
        list.push(3);

        assert_eq!(list.pop(), Some(3));
        assert_eq!(list.pop(), Some(2));

        list.push(4);
        list.push(5);

        assert_eq!(list.pop(), Some(5));
        assert_eq!(list.pop(), Some(4));

        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), None);
    }

    #[test]
    fn peek() {
        //
        let mut list = List::new();
        assert_eq!(list.peek(), None);
        assert_eq!(list.peek_mut(), None);

        list.push(1);
        list.push(2);
        list.push(3);

        assert_eq!(list.peek(), Some(&3));
        assert_eq!(list.peek_mut(), Some(&mut 3));
    }

    #[test]
    fn into_iter() {
        let mut list = List::new();
        list.push(1);
        list.push(2);
        list.push(3);

        let mut iter = list.into_iter();
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(1));
    }

    #[test]
    fn iter() {
        let mut list = List::new();
        list.push(1);
        list.push(2);
        list.push(3);

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&1));
    }

    #[test]
    fn iter_mut() {
        let mut list = List::new();
        list.push(1);
        list.push(2);
        list.push(3);

        let mut iter_mut = list.iter_mut();
        assert_eq!(iter_mut.next(), Some(&mut 3));
        assert_eq!(iter_mut.next(), Some(&mut 2));
        assert_eq!(iter_mut.next(), Some(&mut 1));
    }

}
