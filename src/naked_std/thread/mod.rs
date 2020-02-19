//! Standard library threads.
//!
//! ## The threading model
//!
//! Unlike stdlib OS threads, the threads in naked_std softly hold a name, however we can still
//! assign a custom stack size.
//!
//! Communication between threads can be done through
//! [channels], Rust's message-passing types, along with [other forms of thread
//! synchronization](../../naked_std/sync/index.html) and shared-memory data
//! structures. In particular, types that are guaranteed to be
//! threadsafe are easily shared between threads using the
//! atomically-reference-counted container, [`Arc`].
//!
//! Fatal logic errors in Rust cause *thread panic*, if such a thing happens, the scheduler will be
//! informed, the stack will be freed and the panic message will be passed downstream to the
//! `JoinHandle`. Unlike with userspace programs, panics in naked_std cannot be caught as we do not
//! unwind the stack, however you can still recover from threads panicking, furthermore you can
//! instruct at compile time a panicking strategy through a feature gate which restarts the VM.
//! If the panic is not caught the thread will exit, but the panic may optionally be
//! detected from a different thread with [`join`]. If the main thread panics
//! without the panic being caught, the application will exit with the `Failed` status code.
//!
//! When the main thread dies, other threads will keep on running, however this may change in the
//! future.
//!
//! ## Spawning a thread
//!
//! A new thread can be spawned using the [`thread::spawn`][`spawn`] function:
//!
//! ```rust
//! use naked_std::thread;
//!
//! thread::spawn(move || {
//!     // some work here
//! });
//! ```
//!
//! In this example, the spawned thread is "detached" from the current
//! thread. This means that it can outlive its parent (the thread that spawned
//! it), unless this parent is the main thread.
//!
//! The parent thread can also wait on the completion of the child
//! thread; a call to [`spawn`] produces a [`JoinHandle`], which provides
//! a `join` method for waiting:
//!
//! ```rust
//! use naked_std::thread;
//!
//! let child = thread::spawn(move || {
//!     // some work here
//! });
//! // some work here
//! let res = child.join();
//! ```
//!
//! The [`join`] method returns a [`Result<T, String>`] containing [`Ok`] of the final
//! value produced by the child thread, or [`Err`] of the value given to
//! a call to [`panic!`] if the child panicked.
//!
//! ## Configuring threads
//!
//! A new thread can be configured before it is spawned via the [`Builder`] type,
//! which currently allows you to set the name and stack size for the child thread:
//!
//! ```rust
//! use naked_std::thread;
//!
//! thread::Builder::new().name("child1".to_string()).spawn(move || {
//!     println!("Hello, world!");
//! });
//! ```
//!
//! ## The `Thread` type
//!
//! Threads are represented via the [`Thread`] type, which you can get in only one way at the
//! moment:
//!
//! * By requesting the current thread, using the [`thread::current`] function.
//!
//! ## Thread-local storage
//!
//! Thread-local storage has not been implemented.
//!
//! ## Naming threads
//!
//! Threads are able to have associated names for identification purposes. By default, spawned
//! threads are unnamed. To specify a name for a thread, build the thread with [`Builder`] and pass
//! the desired thread name to [`Builder::name`]. To retrieve the thread name from within the
//! thread, use [`Thread::name`]. A couple examples of where the name of a thread gets used:
//!
//! * If a panic occurs in a named thread, the thread name will be printed in the panic message.
//!
//! ## Stack size
//!
//! The default stack size for spawned threads is 4 MiB, though this particular stack size is
//! subject to change in the future. You can manually specify the stack size for spawned threads by
//! building the thread with [`Builder`] and passing the desired stack size to [`Builder::stack_size`].
//!
//! [channels]: ../../naked_std/sync/mpsc/index.html
//! [`Arc`]: ../../naked_std/sync/struct.Arc.html
//! [`spawn`]: ../../naked_std/thread/fn.spawn.html
//! [`JoinHandle`]: ../../naked_std/thread/struct.JoinHandle.html
//! [`join`]: ../../naked_std/thread/struct.JoinHandle.html#method.join
//! [`Result`]: ../../naked_std/result/enum.Result.html
//! [`Ok`]: ../../naked_std/result/enum.Result.html#variant.Ok
//! [`Err`]: ../../naked_std/result/enum.Result.html#variant.Err
//! [`panic!`]: ../../naked_std/macro.panic.html
//! [`Builder`]: ../../naked_std/thread/struct.Builder.html
//! [`Builder::stack_size`]: ../../naked_std/thread/struct.Builder.html#method.stack_size
//! [`Builder::name`]: ../../naked_std/thread/struct.Builder.html#method.name
//! [`thread::current`]: ../../naked_std/thread/fn.current.html
//! [`Thread`]: ../../naked_std/thread/struct.Thread.html
//! [`park`]: ../../naked_std/thread/fn.park.html
//! [`Thread::name`]: ../../naked_std/thread/struct.Thread.html#method.name
//! [`Cell`]: ../cell/struct.Cell.html
//! [`RefCell`]: ../cell/struct.RefCell.html

use crate::{
    arch::{
        memory::{alloc_stack, StackBounds},
        pit::get_milis,
    },
    cell::UnsafeCell,
    prelude::*,
    schedule as scheduler,
    schedule::stack::Stack,
    sync::{atomic::{AtomicBool, AtomicU64, Ordering},
    Arc},
};
use x86_64::{structures::paging::mapper, VirtAddr};

/// Thread factory, which can be used in order to configure the properties of
/// a new thread.
///
/// Methods can be chained on it in order to configure it.
///
/// The two configurations available are:
///
/// - [`name`]: specifies an [associated name for the thread][naming-threads]
/// - [`stack_size`]: specifies the [desired stack size for the thread][stack-size]
///
/// The [`spawn`] method will take ownership of the builder and create an thread handle
/// with the given configuration.
///
/// The [`thread::spawn`] free function uses a `Builder` with default values and returns the
/// [JoinHandle].
///
/// # Examples
///
/// ```
/// use naked_std::thread;
///
/// let builder = thread::Builder::new();
///
/// let handler = builder.spawn(|| {
///     // thread code
/// }).unwrap();
///
/// handler.join().unwrap();
/// ```
///
/// [`thread::spawn`]: ../../naked_std/thread/fn.spawn.html
/// [`stack_size`]: ../../naked_std/thread/struct.Builder.html#method.stack_size
/// [`name`]: ../../naked_std/thread/struct.Builder.html#method.name
/// [`spawn`]: ../../naked_std/thread/struct.Builder.html#method.spawn
/// [`unwrap`]: ../../naked_std/result/enum.Result.html#method.unwrap
/// [naming-threads]: ./index.html#naming-threads
/// [stack-size]: ./index.html#stack-size
#[derive(Debug)]
pub struct Builder {
    /// A name assigned to a thread that can be used for identification in panic messages
    name: Option<String>,
    /// The desired stack size to be assigned to the thread
    stack_size: Option<u64>,
}

impl Builder {
    /// Generates the base configuration for spawning a thread, from which
    /// configuration methods can be chained.
    ///
    /// # Examples
    ///
    /// ```
    /// use naked_std::thread;
    ///
    /// let builder = thread::Builder::new()
    ///                               .name("foo".into())
    ///                               .stack_size(32 * 1024);
    ///
    /// let handler = builder.spawn(|| {
    ///     // thread code
    /// }).unwrap();
    ///
    /// handler.join().unwrap();
    /// ```
    pub fn new() -> Self {
        Self {
            name: None,
            stack_size: None,
        }
    }

    /// Names the thread-to-be. Currently the name is used for identification
    /// only in panic messages.
    ///
    /// For more information about named threads, see
    /// [this module-level documentation][naming-threads].
    ///
    /// # Examples
    ///
    /// ```
    /// use naked_std::thread;
    ///
    /// let builder = thread::Builder::new()
    ///     .name("foo".into());
    ///
    /// handler.join().unwrap();
    /// ```
    ///
    /// [naming-threads]: ./index.html#naming-threads
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the size of the stack (in pages of 4KiB) for the new thread.
    ///
    /// For more information about the stack size for threads, see
    /// [this module-level documentation][stack-size].
    ///
    /// # Examples
    ///
    /// ```
    /// use naked_std::thread;
    ///
    /// let builder = thread::Builder::new().stack_size(4);
    /// ```
    ///
    /// [stack-size]: ./index.html#stack-size
    pub fn stack_size(mut self, stack_size: u64) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    /// Spawns a new thread by taking ownership of the `Builder`, and returns an
    /// [`JoinHandle`].
    ///
    /// The spawned thread may outlive the caller. The join handle can
    /// be used to block on termination of the child thread, including
    /// recovering its panics.
    ///
    /// For a more complete documentation see [`thread::spawn`][`spawn`].
    ///
    /// [`spawn`]: ../../std/thread/fn.spawn.html
    /// [`JoinHandle`]: ../../std/thread/struct.JoinHandle.html
    ///
    /// # Panics
    /// Panics if the low-level methods that set up the threads return a Err.
    ///
    /// # Examples
    ///
    /// ```
    /// use naked_std::thread;
    ///
    /// let builder = thread::Builder::new();
    ///
    /// let handler = builder.spawn(|| {
    ///     // thread code
    /// }).unwrap();
    ///
    /// handler.join().unwrap();
    /// ```
    pub fn spawn<F, T>(self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let handle: JoinHandle<T> = JoinHandle::new();
        let mut switch = handle.get_switch();
        let panic_state = handle.get_panic();
        let inner = handle.get_inner();

        let thread = Thread::new(
            move || {
                unsafe {
                    *inner.0.get() = Some(f());
                    Arc::get_mut_unchecked(&mut switch).switch();
                }

                scheduler::remove_self();

                loop {
                    x86_64::instructions::hlt();
                }
            },
            self.stack_size.unwrap_or(2),
            panic_state,
            handle.get_switch(),
        );

        unsafe {
            scheduler::add_new_thread(thread.unwrap());
        }

        handle
    }
}

/// Spawns a new thread, returning a [`JoinHandle`] for it.
///
/// The join handle will implicitly *detach* the child thread upon being
/// dropped. In this case, the child thread may outlive the parent.
/// Additionally, the join handle provides a [`join`] method that can be used
/// to join the child thread. If the child thread panics, [`join`] will return
/// an [`Err`] containing the argument given to [`panic`].
///
/// This will create a thread using default parameters of [`Builder`], if you
/// want to specify the stack size or the name of the thread, use this API
/// instead.
///
/// As you can see in the signature of `spawn` there are two constraints on
/// both the closure given to `spawn` and its return value, let's explain them:
///
/// - The `'static` constraint means that the closure and its return value
///   must have a lifetime of the whole program execution. The reason for this
///   is that threads can `detach` and outlive the lifetime they have been
///   created in.
///   Indeed if the thread, and by extension its return value, can outlive their
///   caller, we need to make sure that they will be valid afterwards, and since
///   we *can't* know when it will return we need to have them valid as long as
///   possible, that is until the end of the program, hence the `'static`
///   lifetime.
/// - The [`Send`] constraint is because the closure will need to be passed
///   *by value* from the thread where it is spawned to the new thread. Its
///   return value will need to be passed from the new thread to the thread
///   where it is `join`ed.
///   As a reminder, the [`Send`] marker trait expresses that it is safe to be
///   passed from thread to thread. [`Sync`] expresses that it is safe to have a
///   reference be passed from thread to thread.
///
/// # Panics
///
/// Panics if the OS fails to create a thread.
///
/// # Examples
///
/// Creating a thread.
///
/// ```
/// use naked_std::thread;
///
/// let handler = thread::spawn(|| {
///     // thread code
/// });
///
/// handler.join().unwrap();
/// ```
///
/// As mentioned in the module documentation, threads are usually made to
/// communicate using [`channels`], here is how it usually looks.
///
/// This example also shows how to use `move`, in order to give ownership
/// of values to a thread.
///
/// ```
/// use naked_std::thread;
/// use naked_std::sync::mpsc::channel;
///
/// let (tx, rx) = channel();
///
/// let sender = thread::spawn(move || {
///     tx.send("Hello, thread".to_owned())
///         .expect("Unable to send on channel");
/// });
///
/// let receiver = thread::spawn(move || {
///     let value = rx.recv().expect("Unable to receive from channel");
///     println!("{}", value);
/// });
///
/// sender.join().expect("The sender thread has panicked");
/// receiver.join().expect("The receiver thread has panicked");
/// ```
///
/// A thread can also return a value through its [`JoinHandle`], you can use
/// this to make asynchronous computations.
///
/// ```
/// use naked_std::thread;
///
/// let computation = thread::spawn(|| {
///     // Some expensive computation.
///     42
/// });
///
/// let result = computation.join().unwrap();
/// println!("{}", result);
/// ```
///
/// [`channels`]: ../../naked_std/sync/mpsc/index.html
/// [`JoinHandle`]: ../../naked_std/thread/struct.JoinHandle.html
/// [`join`]: ../../naked_std/thread/struct.JoinHandle.html#method.join
/// [`Err`]: ../../naked_std/result/enum.Result.html#variant.Err
/// [`panic`]: ../../naked_std/macro.panic.html
/// [`Builder::spawn`]: ../../naked_std/thread/struct.Builder.html#method.spawn
/// [`Builder`]: ../../naked_std/thread/struct.Builder.html
/// [`Send`]: ../../naked_std/marker/trait.Send.html
/// [`Sync`]: ../../naked_std/marker/trait.Sync.html
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    Builder::new().spawn(f)
}

/// Gets the ID of the current thread.
///
/// # Examples
///
/// Getting a the id of the current thread with `thread::current()`:
///
/// ```
/// use naked_std::thread;
///
/// let handler = thread::Builder::new()
///     .name("named thread".into())
///     .spawn(|| {
///         let id = thread::current();
///         println!("{}", id);
///     })
///     .unwrap();
///
/// handler.join().unwrap();
/// ```
pub fn current() -> ThreadId {
    scheduler::current_thread_id()
}

/// Cooperatively gives up a timeslice to the OS scheduler.
///
/// This is used when the programmer knows that the thread will have nothing
/// to do for some time, and thus avoid wasting computing time.
///
/// For example when polling on a resource, it is common to check that it is
/// available, and if not to yield in order to avoid busy waiting.
///
/// Thus the pattern of `yield`ing after a failed poll is rather common when
/// implementing low-level shared resources or synchronization primitives.
///
/// However programmers will usually prefer to use [`channel`]s, [`Condvar`]s,
/// [`Mutex`]es or [`join`] for their synchronization routines, as they avoid
/// thinking about thread scheduling.
///
/// Note that [`channel`]s for example are implemented using this primitive.
/// Indeed when you call `send` or `recv`, which are blocking, they will yield
/// if the channel is not available.
///
/// # Examples
///
/// ```
/// use naked_std::thread;
///
/// thread::yield_now();
/// ```
///
/// [`channel`]: ../../naked_std/sync/mpsc/index.html
/// [`spawn`]: ../../naked_std/thread/fn.spawn.html
/// [`join`]: ../../naked_std/thread/struct.JoinHandle.html#method.join
/// [`Mutex`]: ../../naked_std/sync/struct.Mutex.html
/// [`Condvar`]: ../../naked_std/sync/struct.Condvar.html
pub fn yield_now() {
    scheduler::yield_now()
}

/// Determines whether the current thread is unwinding because of panic but in reality
/// this returns false because there is no stack_unwinding, resources are freed forcbily
/// by the scheduler, therefore panicking threads dont need to implement Drop.
///
/// However this may be problematic for implementations like Mutex's which require the
/// detection of panics.
pub fn panicking() -> bool {
    false
}

/// Puts the current thread to sleep for at least the specified amount of time in miliseconds.
///
/// The thread may sleep longer than the duration specified due to scheduling
/// specifics. It will never sleep less.
///
/// # Examples
///
/// ```no_run
/// use vallicks::arch::pit::get_milis;
/// use naked_std::thread;
///
/// let now = get_milis();
///
/// thread::sleep(10);
///
/// assert!(get_milis() >= now);
/// ```
pub fn sleep(ms: u64) {
    scheduler::park_current(ms);
}

/// A unique identifier for a running thread.
///
/// A `ThreadId` is an opaque object that has a unique value for each thread
/// that creates one. `ThreadId`s are also used internally by the unikernel scheduler.
///
/// # Examples
///
/// ```
/// use naked_std::thread;
///
/// let other_thread = thread::spawn(|| {
///     thread::current()
/// });
///
/// let other_thread_id = other_thread.join().unwrap();
/// assert!(thread::current() != other_thread_id);
/// ```
///
/// [`Thread`]: ../../naked_std/thread/struct.Thread.html
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId(u64);

impl ThreadId {
    /// Method generates a unique ID for a thread to be spawned
    fn new() -> Self {
        static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);
        ThreadId(NEXT_THREAD_ID.fetch_add(1, Ordering::SeqCst))
    }

    /// Method is used to create a default ID used internally by some panic guarantees
    pub(crate) fn default() -> Self {
        ThreadId(0)
    }

    /// Returns the ID as a u64
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// This packet is used to communicate the return value between the child thread
/// and the parent thread. Memory is shared through the `Arc` within and there's
/// no need for a mutex here because synchronization happens with `join()` (the
/// parent thread never reads this packet until the child has exited).
///
/// This packet itself is then stored into a `JoinInner` which in turns is placed
/// in `JoinHandle` and `JoinGuard`. Due to the usage of `UnsafeCell` we need to
/// manually worry about impls like Send and Sync. The type `T` should
/// already always be Send (otherwise the thread could not have been created) and
/// this type is inherently Sync because no methods take &self. Regardless,
/// however, we add inheriting impls for Send/Sync to this type to ensure it's
/// Send/Sync.
pub(crate) struct Packet<T>(Arc<UnsafeCell<Option<T>>>);

impl<T> Packet<T> {
    /// Creates a new Packet instance with the default value None
    fn new() -> Self {
        Self(Arc::new(UnsafeCell::new(None)))
    }
}

impl<T> Clone for Packet<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

unsafe impl<T: Send> Send for Packet<T> {}
unsafe impl<T: Sync> Sync for Packet<T> {}

/// This switch is used sort of like a remote trigger to mark an event. We use this switch
/// exclusively within JoinHandle to allow for two things:
/// * The thread itself to mark that it is done executing and that it can be joined now
/// * The scheduler to mark this thread as panicking
///
/// This implementation is inherently safe due to the usafe of the AtomicBool.
pub(crate) struct Switch(AtomicBool);

impl Switch {
    /// Method creates a new switch with the default value true
    fn new() -> Self {
        Self(AtomicBool::new(true))
    }

    /// Method flips the switch, turning the inner value from true to false
    fn switch(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }

    /// Method returns the inner value
    fn is_alive(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// An owned permission to join on a thread (block on its termination).
///
/// A `JoinHandle` *detaches* the associated thread when it is dropped, which
/// means that there is no longer any handle to thread and no way to `join`
/// on it.
///
/// This `struct` is created by the [`thread::spawn`] function and the
/// [`thread::Builder::spawn`] method.
///
/// # Examples
///
/// Creation from [`thread::spawn`]:
///
/// ```
/// use naked_std::thread;
///
/// let join_handle: thread::JoinHandle<_> = thread::spawn(|| {
///     // some work here
/// });
/// ```
///
/// Creation from [`thread::Builder::spawn`]:
///
/// ```
/// use naked_std::thread;
///
/// let builder = thread::Builder::new();
///
/// let join_handle: thread::JoinHandle<_> = builder.spawn(|| {
///     // some work here
/// }).unwrap();
/// ```
///
/// [`thread::spawn`]: fn.spawn.html
/// [`thread::Builder::spawn`]: struct.Builder.html#method.spawn
pub struct JoinHandle<T> {
    /// This is the main switch that allows a thread to mark when it dies.
    alive: Arc<Switch>,
    /// This is the packet/channel over which the thread will send its return value
    inner: Packet<T>,
    /// This is the packet which allows the panic handler to send the panic info.
    panic_state: Packet<String>,
}

impl<T> JoinHandle<T> {
    /// Creates a new emtpy JoinHandle
    pub fn new() -> Self {
        Self {
            alive: Arc::new(Switch::new()),
            inner: Packet::new(),
            panic_state: Packet::new(),
        }
    }

    /// Waits for the associated thread to finish.
    ///
    /// If the child thread panics, [`Err`] is returned with the message given
    /// to [`panic`].
    ///
    /// [`Err`]: ../../naked_std/result/enum.Result.html#variant.Err
    /// [`panic`]: ../../naked_std/macro.panic.html
    ///
    /// # Examples
    ///
    /// ```
    /// use naked_std::thread;
    ///
    /// let builder = thread::Builder::new();
    ///
    /// let join_handle: thread::JoinHandle<_> = builder.spawn(|| {
    ///     // some work here
    /// }).unwrap();
    /// join_handle.join().expect("Couldn't join on the associated thread");
    /// ```
    pub fn join(self) -> Result<T, String> {
        loop {
            if !self.alive.is_alive() {
                match unsafe { (*self.panic_state.0.get()).take() } {
                    Some(x) => return Err(x),
                    None => return unsafe { Ok((*self.inner.0.get()).take().unwrap()) },
                }
            }
        }
    }

    /// Method returns a new copy of the inner channel over which the thread will send its return
    /// value. This function is inherently unsafe as having multiple copies will cause UB, but this
    /// function only ever gets called once.
    fn get_inner(&self) -> Packet<T> {
        self.inner.clone()
    }

    /// Method returns a packet channel specifically intended for the panic handler and scheduler
    /// to send panic info and messages downstream which get returned ass Err()
    fn get_panic(&self) -> Packet<String> {
        self.panic_state.clone()
    }

    /// Switch given to a thread and the scheduler to mark JoinHandle's as joinable. Once the
    /// switch is flipped, the join method will return.
    fn get_switch(&self) -> Arc<Switch> {
        self.alive.clone()
    }
}

/// This struct is the basic building block for a thread, it holds key information to be used by
/// the scheduler to execute our functions.
///
/// This `struct` is created by the [`thread::spawn`] function and the
/// [`thread::Builder::spawn`] method, then it gets passed to the scheduler.
///
/// While the end user will never manually interact with this struct it is important to understand
/// how it works.
///
/// When [`thread::Builder::spawn`] is called, the closure passed to it will get passed onto
/// [`thread::Thread::new`], the constructor will then allocate a new stack bound for the new
/// thread, it will then assign the start of the bound as the stack pointer.
///
/// [`thread::spawn`]: fn.spawn.html
/// [`thread::Builder::spawn`]: struct.Builder.html#method.spawn
/// [`thread::Thread::new`]: struct.Thread.html#method.new
pub struct Thread {
    /// The ID of the thread about to be spawned
    id: ThreadId,
    /// This field is used by the scheduler to understand wether the thread is suposed to be still
    /// asleep, this tuple holds when the thread was parked and for how long
    parked: Option<(u64, u64)>,
    /// The stack pointer that later is used for context switching
    stack_pointer: Option<VirtAddr>,
    /// The start and end of the stack
    stack_bounds: Option<StackBounds>,
    /// This packet is used to send in case of a panic the panic info downstream to the JoinHandle
    panic_state: Packet<String>,
    /// This switch allows the Thread object to remote trigger the JoinHandle to become joinable
    switch: Arc<Switch>,
}

impl Thread {
    /// This method creates a new thread object and begins setting up and preparing the stack for
    /// execution.
    fn new<F>(
        closure: F,
        stack_size: u64,
        panic_state: Packet<String>,
        switch: Arc<Switch>,
    ) -> Result<Self, mapper::MapToError>
    where
        F: FnOnce() -> !,
        F: Send + 'static,
    {
        let mut mapper = crate::globals::MAPPER.lock();
        let mut frame_allocator = crate::globals::FRAME_ALLOCATOR.lock();

        let stack_bounds = alloc_stack(
            stack_size,
            mapper.as_mut().unwrap(),
            frame_allocator.as_mut().unwrap(),
        )?;
        let mut stack = unsafe { Stack::new(stack_bounds.end()) };

        println!(
            "scheduler: new thread stack @ {:#x}..{:#x}",
            stack_bounds.start().as_u64(),
            stack_bounds.end().as_u64()
        );

        stack.set_up_for_closure(Box::new(closure));

        Ok(Self {
            id: ThreadId::new(),
            parked: None,
            stack_pointer: Some(stack.get_stack_pointer()),
            stack_bounds: Some(stack_bounds),
            panic_state,
            switch,
        })
    }

    /// Method creates a blank empty root thread used by the scheduler
    pub(crate) fn create_root_thread() -> Self {
        Self {
            id: ThreadId(0),
            parked: None,
            stack_pointer: None,
            stack_bounds: None,
            panic_state: Packet::new(), // we dont actually care
            switch: Arc::new(Switch::new()),
        }
    }

    /// Returns the id of this thread
    pub fn id(&self) -> ThreadId {
        self.id
    }

    /// Returns the stack pointer for this thread
    pub(crate) fn stack_pointer(&mut self) -> &mut Option<VirtAddr> {
        &mut self.stack_pointer
    }

    /// Returns whether this thread is ready to be unparked or not
    pub fn is_ready(&mut self) -> bool {
        if let Some((parked_at, for_milis)) = self.parked {
            if get_milis() < parked_at + for_milis {
                return false;
            }
            self.parked = None;
        }
        true
    }

    /// Parks this thread for `milis` number of miliseconds
    pub fn park(&mut self, milis: u64) {
        self.parked = Some((get_milis(), milis));
    }

    /// Functions marks this thread as panicking then propagates the `reason` downstream to the
    /// `JoinHandle`
    pub(crate) fn set_panicking(&mut self, reason: String) {
        unsafe {
            *self.panic_state.0.get() = Some(reason);
            Arc::get_mut_unchecked(&mut self.switch).switch();
        }
    }
}

impl core::fmt::Debug for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Thread {{ id: {:?}, parked: {:?}, stack_pointer: {:?}, stack_bounds: {:?} }}",
            self.id, self.parked, self.stack_pointer, self.stack_bounds
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Builder;
    use crate::alloc::string::*;
    use crate::naked_std::sync::mpsc::{channel, Sender};
    use crate::naked_std::thread::{self, ThreadId};
    use crate::prelude::*;
    use core::any::Any;
    use core::mem;
    use core::result;
    use core::u32;

    // !!! These tests are dangerous. If something is buggy, they will hang, !!!
    // !!! instead of exiting cleanly. This might wedge the buildbots.       !!!

    /*
    fn test_unnamed_thread() {
        thread::spawn(move || {
            assert!(thread::current().name().is_none());
        })
        .join()
        .ok()
        .expect("thread panicked");
    }

    fn test_named_thread() {
        Builder::new()
            .name("ada lovelace".to_string())
            .spawn(move || {
                assert!(thread::current().name().unwrap() == "ada lovelace".to_string());
            })
            .unwrap()
            .join()
            .unwrap();
    }
    */

    #[unittest]
    fn test_run_basic() {
        let (tx, rx) = channel();
        thread::spawn(move || {
            tx.send(()).unwrap();
        });
        rx.recv().unwrap();
    }

    #[unittest]
    fn test_join_panic() {
        match thread::spawn(move || panic!()).join() {
            result::Result::Err(_) => (),
            result::Result::Ok(()) => panic!(),
        }
    }

    #[unittest]
    fn test_spawn_sched() {
        let (tx, rx) = channel();

        fn f(i: i32, tx: Sender<()>) {
            let tx = tx.clone();
            thread::spawn(move || {
                if i == 0 {
                    tx.send(()).unwrap();
                } else {
                    f(i - 1, tx);
                }
            });
        }
        f(10, tx);
        rx.recv().unwrap();
    }

    #[unittest]
    fn test_spawn_sched_childs_on_default_sched() {
        let (tx, rx) = channel();

        thread::spawn(move || {
            thread::spawn(move || {
                tx.send(()).unwrap();
            });
        });

        rx.recv().unwrap();
    }

    fn avoid_copying_the_body<F>(spawnfn: F)
    where
        F: FnOnce(Box<dyn Fn() + Send>),
    {
        let (tx, rx) = channel();

        let x: Box<_> = box 1;
        let x_in_parent = (&*x) as *const i32 as usize;

        spawnfn(Box::new(move || {
            let x_in_child = (&*x) as *const i32 as usize;
            tx.send(x_in_child).unwrap();
        }));

        let x_in_child = rx.recv().unwrap();
        assert_eq!(x_in_parent, x_in_child);
    }

    #[unittest]
    fn test_avoid_copying_the_body_spawn() {
        avoid_copying_the_body(|v| {
            thread::spawn(move || v());
        });
    }

    #[unittest]
    fn test_avoid_copying_the_body_thread_spawn() {
        avoid_copying_the_body(|f| {
            thread::spawn(move || {
                f();
            });
        })
    }

    #[unittest]
    fn test_avoid_copying_the_body_join() {
        avoid_copying_the_body(|f| {
            let _ = thread::spawn(move || f()).join();
        })
    }

    #[unittest]
    fn test_child_doesnt_ref_parent() {
        // If the child refcounts the parent thread, this will stack overflow when
        // climbing the thread tree to dereference each ancestor. (See #1789)
        // (well, it would if the constant were 8000+ - I lowered it to be more
        // valgrind-friendly. try this at home, instead..!)
        const GENERATIONS: u32 = 16;
        fn child_no(x: u32) -> Box<dyn Fn() + Send> {
            return Box::new(move || {
                if x < GENERATIONS {
                    thread::spawn(move || child_no(x + 1)());
                }
            });
        }
        thread::spawn(|| child_no(0)());
    }

    #[unittest]
    fn test_simple_newsched_spawn() {
        thread::spawn(move || {});
    }

    #[unittest]
    fn sleep_ms_smoke() {
        thread::sleep(20);
    }

    #[unittest]
    fn test_thread_id_equal() {
        assert!(thread::current() == thread::current());
    }

    #[unittest]
    fn test_thread_id_not_equal() {
        let spawned_id = thread::spawn(|| thread::current()).join().unwrap();
        assert!(thread::current() != spawned_id);
    }
}
