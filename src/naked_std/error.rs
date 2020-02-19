//! Traits for working with Errors shamelessly ported from libstd.

// A note about crates and the facade:
//
// Originally, the `Error` trait was defined in libcore, and the impls
// were scattered about. However, coherence objected to this
// arrangement, because to create the blanket impls for `Box` required
// knowing that `&str: !Error`, and we have no means to deal with that
// sort of conflict just now. Therefore, for the time being, we have
// moved the `Error` trait into libstd. As we evolve a sol'n to the
// coherence challenge (e.g., specialization, neg impls, etc) we can
// reconsider what crate these items belong in.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use core::any::TypeId;
use core::fmt::{Debug, Display};
use core::mem::transmute;
use core::str;

/// `Error` is a trait representing the basic expectations for error values,
/// i.e., values of type `E` in [`Result<T, E>`]. Errors must describe
/// themselves through the [`Display`] and [`Debug`] traits, and may provide
/// cause chain information:
///
/// The [`source`] method is generally used when errors cross "abstraction
/// boundaries". If one module must report an error that is caused by an error
/// from a lower-level module, it can allow access to that error via the
/// [`source`] method. This makes it possible for the high-level module to
/// provide its own errors while also revealing some of the implementation for
/// debugging via [`source`] chains.
///
/// [`Result<T, E>`]: ../result/enum.Result.html
/// [`Display`]: ../fmt/trait.Display.html
/// [`Debug`]: ../fmt/trait.Debug.html
/// [`source`]: trait.Error.html#method.source
pub trait Error: Debug + Display {
    /// The lower-level source of this error, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::fmt;
    ///
    /// #[derive(Debug)]
    /// struct SuperError {
    ///     side: SuperErrorSideKick,
    /// }
    ///
    /// impl fmt::Display for SuperError {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f, "SuperError is here!")
    ///     }
    /// }
    ///
    /// impl Error for SuperError {
    ///     fn source(&self) -> Option<&(dyn Error + 'static)> {
    ///         Some(&self.side)
    ///     }
    /// }
    ///
    /// #[derive(Debug)]
    /// struct SuperErrorSideKick;
    ///
    /// impl fmt::Display for SuperErrorSideKick {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f, "SuperErrorSideKick is here!")
    ///     }
    /// }
    ///
    /// impl Error for SuperErrorSideKick {}
    ///
    /// fn get_super_error() -> Result<(), SuperError> {
    ///     Err(SuperError { side: SuperErrorSideKick })
    /// }
    ///
    /// fn main() {
    ///     match get_super_error() {
    ///         Err(e) => {
    ///             println!("Error: {}", e.description());
    ///             println!("Caused by: {}", e.source().unwrap());
    ///         }
    ///         _ => println!("No error"),
    ///     }
    /// }
    /// ```
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    /// Gets the `TypeId` of `self`.
    #[doc(hidden)]
    fn type_id(&self, _: private::Internal) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }

    /// ```
    /// if let Err(e) = "xc".parse::<u32>() {
    ///     // Print `e` itself, no need for description().
    ///     eprintln!("Error: {}", e);
    /// }
    /// ```
    fn description(&self) -> &str {
        "this method is deprecated, use Display instead"
    }
}

mod private {
    // This is a hack to prevent `type_id` from being overridden by `Error`
    // implementations, since that can enable unsound downcasting.
    #[derive(Debug)]
    pub struct Internal;
}

impl<'a, E: Error + 'a> From<E> for Box<dyn Error + 'a> {
    /// Converts a type of [`Error`] into a box of dyn [`Error`].
    ///
    /// [`Error`]: ../error/trait.Error.html
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::fmt;
    /// use std::mem;
    ///
    /// #[derive(Debug)]
    /// struct AnError;
    ///
    /// impl fmt::Display for AnError {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f , "An error")
    ///     }
    /// }
    ///
    /// impl Error for AnError {}
    ///
    /// let an_error = AnError;
    /// assert!(0 == mem::size_of_val(&an_error));
    /// let a_boxed_error = Box::<dyn Error>::from(an_error);
    /// assert!(mem::size_of::<Box<dyn Error>>() == mem::size_of_val(&a_boxed_error))
    /// ```
    fn from(err: E) -> Box<dyn Error + 'a> {
        Box::new(err)
    }
}

impl<'a, E: Error + Send + Sync + 'a> From<E> for Box<dyn Error + Send + Sync + 'a> {
    /// Converts a type of [`Error`] + [`Send`] + [`Sync`] into a box of
    /// dyn [`Error`] + [`Send`] + [`Sync`].
    ///
    /// [`Error`]: ../error/trait.Error.html
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::fmt;
    /// use std::mem;
    ///
    /// #[derive(Debug)]
    /// struct AnError;
    ///
    /// impl fmt::Display for AnError {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f , "An error")
    ///     }
    /// }
    ///
    /// impl Error for AnError {}
    ///
    /// unsafe impl Send for AnError {}
    ///
    /// unsafe impl Sync for AnError {}
    ///
    /// let an_error = AnError;
    /// assert!(0 == mem::size_of_val(&an_error));
    /// let a_boxed_error = Box::<dyn Error + Send + Sync>::from(an_error);
    /// assert!(
    ///     mem::size_of::<Box<dyn Error + Send + Sync>>() == mem::size_of_val(&a_boxed_error))
    /// ```
    fn from(err: E) -> Box<dyn Error + Send + Sync + 'a> {
        Box::new(err)
    }
}

impl<'a> From<&str> for Box<dyn Error + Send + Sync + 'a> {
    /// Converts a [`str`] into a box of dyn [`Error`] + [`Send`] + [`Sync`].
    ///
    /// [`Error`]: ../error/trait.Error.html
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::mem;
    ///
    /// let a_str_error = "a str error";
    /// let a_boxed_error = Box::<dyn Error + Send + Sync>::from(a_str_error);
    /// assert!(
    ///     mem::size_of::<Box<dyn Error + Send + Sync>>() == mem::size_of_val(&a_boxed_error))
    /// ```
    #[inline]
    fn from(err: &str) -> Box<dyn Error + Send + Sync + 'a> {
        From::from(String::from(err))
    }
}

impl From<&str> for Box<dyn Error> {
    /// Converts a [`str`] into a box of dyn [`Error`].
    ///
    /// [`Error`]: ../error/trait.Error.html
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::mem;
    ///
    /// let a_str_error = "a str error";
    /// let a_boxed_error = Box::<dyn Error>::from(a_str_error);
    /// assert!(mem::size_of::<Box<dyn Error>>() == mem::size_of_val(&a_boxed_error))
    /// ```
    fn from(err: &str) -> Box<dyn Error> {
        From::from(String::from(err))
    }
}

impl<'a, 'b> From<Cow<'b, str>> for Box<dyn Error + Send + Sync + 'a> {
    /// Converts a [`Cow`] into a box of dyn [`Error`] + [`Send`] + [`Sync`].
    ///
    /// [`Cow`]: ../borrow/enum.Cow.html
    /// [`Error`]: ../error/trait.Error.html
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::mem;
    /// use std::borrow::Cow;
    ///
    /// let a_cow_str_error = Cow::from("a str error");
    /// let a_boxed_error = Box::<dyn Error + Send + Sync>::from(a_cow_str_error);
    /// assert!(
    ///     mem::size_of::<Box<dyn Error + Send + Sync>>() == mem::size_of_val(&a_boxed_error))
    /// ```
    fn from(err: Cow<'b, str>) -> Box<dyn Error + Send + Sync + 'a> {
        From::from(String::from(err))
    }
}

impl<'a> From<Cow<'a, str>> for Box<dyn Error> {
    /// Converts a [`Cow`] into a box of dyn [`Error`].
    ///
    /// [`Cow`]: ../borrow/enum.Cow.html
    /// [`Error`]: ../error/trait.Error.html
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::mem;
    /// use std::borrow::Cow;
    ///
    /// let a_cow_str_error = Cow::from("a str error");
    /// let a_boxed_error = Box::<dyn Error>::from(a_cow_str_error);
    /// assert!(mem::size_of::<Box<dyn Error>>() == mem::size_of_val(&a_boxed_error))
    /// ```
    fn from(err: Cow<'a, str>) -> Box<dyn Error> {
        From::from(String::from(err))
    }
}

impl Error for ! {
    fn description(&self) -> &str {
        "! type error"
    }
}

impl<T: Error> Error for Box<T> {
    fn description(&self) -> &str {
        Error::description(&**self)
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Error::source(&**self)
    }
}

impl Error for String {
    fn description(&self) -> &str {
        self.as_str()
    }
}

// Copied from `any.rs`.
impl dyn Error + 'static {
    /// Returns `true` if the boxed type is the same as `T`
    #[inline]
    pub fn is<T: Error + 'static>(&self) -> bool {
        // Get `TypeId` of the type this function is instantiated with.
        let t = TypeId::of::<T>();

        // Get `TypeId` of the type in the trait object.
        let boxed = self.type_id(private::Internal);

        // Compare both `TypeId`s on equality.
        t == boxed
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    #[inline]
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn Error as *const T)) }
        } else {
            None
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    #[inline]
    pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            unsafe { Some(&mut *(self as *mut dyn Error as *mut T)) }
        } else {
            None
        }
    }
}

impl dyn Error + 'static + Send {
    /// Forwards to the method defined on the type `dyn Error`.
    #[inline]
    pub fn is<T: Error + 'static>(&self) -> bool {
        <dyn Error + 'static>::is::<T>(self)
    }

    /// Forwards to the method defined on the type `dyn Error`.
    #[inline]
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        <dyn Error + 'static>::downcast_ref::<T>(self)
    }

    /// Forwards to the method defined on the type `dyn Error`.
    #[inline]
    pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T> {
        <dyn Error + 'static>::downcast_mut::<T>(self)
    }
}

impl dyn Error + 'static + Send + Sync {
    /// Forwards to the method defined on the type `dyn Error`.
    #[inline]
    pub fn is<T: Error + 'static>(&self) -> bool {
        <dyn Error + 'static>::is::<T>(self)
    }

    /// Forwards to the method defined on the type `dyn Error`.
    #[inline]
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        <dyn Error + 'static>::downcast_ref::<T>(self)
    }

    /// Forwards to the method defined on the type `dyn Error`.
    #[inline]
    pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T> {
        <dyn Error + 'static>::downcast_mut::<T>(self)
    }
}

impl dyn Error {
    #[inline]
    /// Attempts to downcast the box to a concrete type.
    pub fn downcast<T: Error + 'static>(self: Box<Self>) -> Result<Box<T>, Box<dyn Error>> {
        if self.is::<T>() {
            unsafe {
                let raw: *mut dyn Error = Box::into_raw(self);
                Ok(Box::from_raw(raw as *mut T))
            }
        } else {
            Err(self)
        }
    }

    /// Returns an iterator starting with the current error and continuing with
    /// recursively calling [`source`].
    ///
    /// If you want to omit the current error and only use its sources,
    /// use `skip(1)`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(error_iter)]
    /// use std::error::Error;
    /// use std::fmt;
    ///
    /// #[derive(Debug)]
    /// struct A;
    ///
    /// #[derive(Debug)]
    /// struct B(Option<Box<dyn Error + 'static>>);
    ///
    /// impl fmt::Display for A {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f, "A")
    ///     }
    /// }
    ///
    /// impl fmt::Display for B {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f, "B")
    ///     }
    /// }
    ///
    /// impl Error for A {}
    ///
    /// impl Error for B {
    ///     fn source(&self) -> Option<&(dyn Error + 'static)> {
    ///         self.0.as_ref().map(|e| e.as_ref())
    ///     }
    /// }
    ///
    /// let b = B(Some(Box::new(A)));
    ///
    /// // let err : Box<Error> = b.into(); // or
    /// let err = &b as &(dyn Error);
    ///
    /// let mut iter = err.chain();
    ///
    /// assert_eq!("B".to_string(), iter.next().unwrap().to_string());
    /// assert_eq!("A".to_string(), iter.next().unwrap().to_string());
    /// assert!(iter.next().is_none());
    /// assert!(iter.next().is_none());
    /// ```
    ///
    /// [`source`]: trait.Error.html#method.source
    #[inline]
    pub fn chain(&self) -> Chain<'_> {
        Chain {
            current: Some(self),
        }
    }
}

/// An iterator over an [`Error`] and its sources.
///
/// If you want to omit the initial error and only process
/// its sources, use `skip(1)`.
///
/// [`Error`]: trait.Error.html
#[derive(Clone, Debug)]
pub struct Chain<'a> {
    current: Option<&'a (dyn Error + 'static)>,
}

impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(Error::source);
        current
    }
}

impl dyn Error + Send {
    #[inline]
    /// Attempts to downcast the box to a concrete type.
    pub fn downcast<T: Error + 'static>(self: Box<Self>) -> Result<Box<T>, Box<dyn Error + Send>> {
        let err: Box<dyn Error> = self;
        <dyn Error>::downcast(err).map_err(|s| unsafe {
            // Reapply the `Send` marker.
            transmute::<Box<dyn Error>, Box<dyn Error + Send>>(s)
        })
    }
}

impl dyn Error + Send + Sync {
    #[inline]
    /// Attempts to downcast the box to a concrete type.
    pub fn downcast<T: Error + 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        let err: Box<dyn Error> = self;
        <dyn Error>::downcast(err).map_err(|s| unsafe {
            // Reapply the `Send + Sync` marker.
            transmute::<Box<dyn Error>, Box<dyn Error + Send + Sync>>(s)
        })
    }
}

#[cfg(not(test))]
mod tests {
    use super::Error;
    use core::fmt;
    use alloc::boxed::Box;

    #[derive(Debug, PartialEq)]
    struct A;
    #[derive(Debug, PartialEq)]
    struct B;

    impl fmt::Display for A {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "A")
        }
    }
    impl fmt::Display for B {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "B")
        }
    }

    impl Error for A {}
    impl Error for B {}

    #[test_case]
    fn downcasting() {
        let mut a = A;
        let a = &mut a as &mut (dyn Error + 'static);
        assert_eq!(a.downcast_ref::<A>(), Some(&A));
        assert_eq!(a.downcast_ref::<B>(), None);
        assert_eq!(a.downcast_mut::<A>(), Some(&mut A));
        assert_eq!(a.downcast_mut::<B>(), None);

        let a: Box<dyn Error> = Box::new(A);
        match a.downcast::<B>() {
            Ok(..) => panic!("expected error"),
            Err(e) => assert_eq!(*e.downcast::<A>().unwrap(), A),
        }
    }
}
