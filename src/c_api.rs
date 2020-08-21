use std::os::raw::{c_char, c_int, c_uint, c_void};

use va_list::VaList;

// ===========================================================================
// Logging

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SayLevel {
    Fatal = 0,
    System = 1,
    Error = 2,
    Crit = 3,
    Warn = 4,
    Info = 5,
    Debug = 6,
}

pub type SayFunc = Option<unsafe extern "C" fn(c_int, *const c_char, c_int, *const c_char, *const c_char, ...)>;

extern "C" {
    #[link_name = "log_level"]
    pub static mut LOG_LEVEL: c_int;

    #[link_name = "_say"]
    pub static mut SAY_FN: SayFunc;
}

// ===========================================================================
// Fiber

/**
 * Fiber - contains information about fiber
 */
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Fiber {
    _unused: [u8; 0]
}

pub type FiberFunc = Option<unsafe extern "C" fn(VaList) -> c_int>;

extern "C" {
    /**
     * Create a new fiber.
     *
     * Takes a fiber from fiber cache, if it's not empty.
     * Can fail only if there is not enough memory for
     * the fiber structure or fiber stack.
     *
     * The created fiber automatically returns itself
     * to the fiber cache when its "main" function
     * completes.
     *
     * \param name       string with fiber name
     * \param FiberFunc func for run inside fiber
     *
     * \sa fiber_start
     */
    pub fn fiber_new(name: *const c_char, f: FiberFunc) -> *mut Fiber;

    /**
     * Return control to another fiber and wait until it'll be woken.
     *
     * \sa fiber_wakeup
     */
    pub fn fiber_yield();

    /**
     * Start execution of created fiber.
     *
     * \param callee fiber to start
     * \param ...    arguments to start the fiber with
     *
     * \sa fiber_new
     */
    pub fn fiber_start(callee: *mut Fiber, ...);

    /**
     * Interrupt a synchronous wait of a fiber
     *
     * \param f fiber to be woken up
     */
    pub fn fiber_wakeup(f: *mut Fiber);

    /**
     * Cancel the subject fiber. (set FIBER_IS_CANCELLED flag)
     *
     * If target fiber's flag FIBER_IS_CANCELLABLE set, then it would
     * be woken up (maybe prematurely). Then current fiber yields
     * until the target fiber is dead (or is woken up by
     * \sa fiber_wakeup).
     *
     * \param f fiber to be cancelled
     */
    pub fn fiber_cancel(f: *mut Fiber);

    /**
     * Make it possible or not possible to wakeup the current
     * fiber immediately when it's cancelled.
     *
     * @param yesno status to set
     * @return previous state.
     */
    pub fn fiber_set_cancellable(yesno: bool) -> bool;

    /**
     * Set fiber to be joinable (false by default).
     * \param yesno status to set
     */
    pub fn fiber_set_joinable(fiber: *mut Fiber, yesno: bool);

    /**
     * Wait until the fiber is dead and then move its execution
     * status to the caller.
     * The fiber must not be detached (@sa fiber_set_joinable()).
     * @pre FIBER_IS_JOINABLE flag is set.
     *
     * \param f fiber to be woken up
     * \return fiber function ret code
     */
    pub fn fiber_join(f: *mut Fiber) -> c_int;

    /**
     * Put the current fiber to sleep for at least 's' seconds.
     *
     * \param s time to sleep
     *
     * \note this is a cancellation point (\sa fiber_is_cancelled)
     */
    pub fn fiber_sleep(s: f64);

    /**
     * Check current fiber for cancellation (it must be checked
     * manually).
     */
    pub fn fiber_is_cancelled() -> bool;

    /**
     * Report loop begin time as double (cheap).
     */
    pub fn fiber_time() -> f64;

    /**
     * Report loop begin time as 64-bit int.
     */
    pub fn fiber_time64() -> u64;

    /**
     * Report loop begin time as double (cheap).
     * Uses monotonic clock.
     */
    pub fn fiber_clock() -> f64;

    /**
     * Report loop begin time as 64-bit int.
     * Uses monotonic clock.
     */
    pub fn fiber_clock64() -> u64;

    /**
     * Reschedule fiber to end of event loop cycle.
     */
    pub fn fiber_reschedule();
}

// ===========================================================================
// Slab cache

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SlabCache {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Return SlabCache suitable to use with tarantool/small library
     */
    pub fn cord_slab_cache() -> *mut SlabCache;
}

// ===========================================================================
// CoIO

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CoioFlags {
    Read = 1,
    Write = 2,
}

extern "C" {
    /**
     * Wait until READ or WRITE event on socket (\a fd). Yields.
     * \param fd - non-blocking socket file description
     * \param events - requested events to wait.
     * Combination of TNT_IO_READ | TNT_IO_WRITE bit flags.
     * \param timeoout - timeout in seconds.
     * \retval 0 - timeout
     * \retval >0 - returned events. Combination of TNT_IO_READ | TNT_IO_WRITE
     * bit flags.
     */
    pub fn coio_wait(fd: c_int, event: c_int, timeout: f64) -> c_int;

    /**
     * Close the fd and wake any fiber blocked in
     * coio_wait() call on this fd.
     */
    pub fn coio_close(fd: c_int) -> c_int;

    /**
     * Create new eio task with specified function and
     * arguments. Yield and wait until the task is complete
     * or a timeout occurs.
     *
     * This function doesn't throw exceptions to avoid double error
     * checking: in most cases it's also necessary to check the return
     * value of the called function and perform necessary actions. If
     * func sets errno, the errno is preserved across the call.
     *
     * @retval -1 and errno = ENOMEM if failed to create a task
     * @retval the function return (errno is preserved).
     *
     * @code
     *	static ssize_t openfile_cb(va_list ap)
     *	{
     *	         const char *filename = va_arg(ap);
     *	         int flags = va_arg(ap);
     *	         return open(filename, flags);
     *	}
     *
     *	if (coio_call(openfile_cb, 0.10, "/tmp/file", 0) == -1)
     *		// handle errors.
     *	...
     * @endcode
     */
    pub fn coio_call(func: Option<unsafe extern "C" fn(VaList) -> c_int>, ...) -> isize;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AddrInfo {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Fiber-friendly version of getaddrinfo(3).
     *
     * @param host host name, i.e. "tarantool.org"
     * @param port service name, i.e. "80" or "http"
     * @param hints hints, see getaddrinfo(3)
     * @param res[out] result, see getaddrinfo(3)
     * @param timeout timeout
     * @retval  0 on success, please free @a res using freeaddrinfo(3).
     * @retval -1 on error, check diag.
     *            Please note that the return value is not compatible with
     *            getaddrinfo(3).
     * @sa getaddrinfo()
     */
    pub fn coio_getaddrinfo(
        host: *const c_char,
        port: *const c_char,
        hints: *const AddrInfo,
        res: *mut *mut AddrInfo,
        timeout: f64
    ) -> c_int;
}

// ===========================================================================
// Transaction

extern "C" {
    /**
     * Return true if there is an active transaction.
     */
    pub fn box_txn() -> bool;

    /**
     * Begin a transaction in the current fiber.
     *
     * A transaction is attached to caller fiber, therefore one fiber can have
     * only one active transaction.
     *
     * @retval 0 - success
     * @retval -1 - failed, perhaps a transaction has already been
     * started
     */
    pub fn box_txn_begin() -> c_int;

    /**
     * Commit the current transaction.
     * @retval 0 - success
     * @retval -1 - failed, perhaps a disk write failure.
     * started
     */
    pub fn box_txn_commit() -> c_int;

    /**
     * Rollback the current transaction.
     * May fail if called from a nested
     * statement.
     */
    pub fn box_txn_rollback() -> c_int;

    /**
     * Allocate memory on txn memory pool.
     * The memory is automatically deallocated when the transaction
     * is committed or rolled back.
     *
     * @retval NULL out of memory
     */
    pub fn box_txn_alloc(size: usize) -> *mut c_void;
}

// ===========================================================================
// Tuple

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BoxTupleFormat {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Tuple Format.
     *
     * Each Tuple has associated format (class). Default format is used to
     * create tuples which are not attach to any particular space.
     */
    pub fn box_tuple_format_default() -> *mut BoxTupleFormat;
}

/**
 * Tuple
 */
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BoxTuple {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Allocate and initialize a new Tuple from a raw MsgPack Array data.
     *
     * \param format Tuple format.
     * Use box_tuple_format_default() to create space-independent Tuple.
     * \param data Tuple data in MsgPack Array format ([field1, field2, ...]).
     * \param end the end of \a data
     * \retval NULL on out of memory
     * \retval Tuple otherwise
     * \pre data, end is valid MsgPack Array
     * \sa \code box.Tuple.new(data) \endcode
     */
    pub fn box_tuple_new(format: *mut BoxTupleFormat, data: *const c_char, end: *const c_char) -> *mut BoxTuple;

    /**
     * Increase the reference counter of Tuple.
     *
     * Tuples are reference counted. All functions that return tuples guarantee
     * that the last returned Tuple is refcounted internally until the next
     * call to API function that yields or returns another Tuple.
     *
     * You should increase the reference counter before taking tuples for long
     * processing in your code. Such tuples will not be garbage collected even
     * if another fiber remove they from space. After processing please
     * decrement the reference counter using box_tuple_unref(), otherwise the
     * Tuple will leak.
     *
     * \param Tuple a Tuple
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa box_tuple_unref()
     */
    pub fn box_tuple_ref(tuple: *mut BoxTuple) -> c_int;

    /**
     * Decrease the reference counter of Tuple.
     *
     * \param Tuple a Tuple
     * \sa box_tuple_ref()
     */
    pub fn box_tuple_unref(tuple: *mut BoxTuple);

    /**
     * Return the number of fields in Tuple (the size of MsgPack Array).
     * \param Tuple a Tuple
     */
    pub fn box_tuple_field_count(tuple: *const BoxTuple) -> u32;

    /**
     * Return the number of bytes used to store internal Tuple data (MsgPack Array).
     * \param Tuple a Tuple
     */
    pub fn box_tuple_bsize(tuple: *const BoxTuple) -> usize;

    /**
     * Dump raw MsgPack data to the memory byffer \a buf of size \a size.
     *
     * Store Tuple fields in the memory buffer.
     * \retval -1 on error.
     * \retval number of bytes written on success.
     * Upon successful return, the function returns the number of bytes written.
     * If buffer size is not enough then the return value is the number of bytes
     * which would have been written if enough space had been available.
     */
    pub fn box_tuple_to_buf(tuple: *const BoxTuple, buf: *mut c_char, size: usize) -> isize;

    /**
     * Return the associated format.
     * \param Tuple Tuple
     * \return TupleFormat
     */
    pub fn box_tuple_format(tuple: *const BoxTuple) -> *mut BoxTupleFormat;

    /**
     * Return the raw Tuple field in MsgPack format.
     *
     * The buffer is valid until next call to box_tuple_* functions.
     *
     * \param Tuple a Tuple
     * \param fieldno zero-based index in MsgPack array.
     * \retval NULL if i >= box_tuple_field_count(Tuple)
     * \retval msgpack otherwise
     */
    pub fn box_tuple_field(tuple: *const BoxTuple, fieldno: u32) -> *const c_char;
}

/**
 * Tuple iterator
 */
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BoxTupleIterator {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Allocate and initialize a new Tuple iterator. The Tuple iterator
     * allow to iterate over fields at root level of MsgPack array.
     *
     * Example:
     * \code
     * box_tuple_iterator *it = box_tuple_iterator(Tuple);
     * if (it == NULL) {
     *      // error handling using box_error_last()
     * }
     * const char *field;
     * while (field = box_tuple_next(it)) {
     *      // process raw MsgPack data
     * }
     *
     * // rewind iterator to first position
     * box_tuple_rewind(it);
     * assert(box_tuple_position(it) == 0);
     *
     * // rewind iterator to first position
     * field = box_tuple_seek(it, 3);
     * assert(box_tuple_position(it) == 4);
     *
     * box_iterator_free(it);
     * \endcode
     *
     * \post box_tuple_position(it) == 0
     */
    pub fn box_tuple_iterator(tuple: *mut BoxTuple) -> *mut BoxTupleIterator;

    /**
     * Destroy and free Tuple iterator
     */
    pub fn box_tuple_iterator_free(it: *mut BoxTupleIterator);

    /**
     * Return zero-based next position in iterator.
     * That is, this function return the field id of field that will be
     * returned by the next call to box_tuple_next(it). Returned value is zero
     * after initialization or rewind and box_tuple_field_count(Tuple)
     * after the end of iteration.
     *
     * \param it Tuple iterator
     * \returns position.
     */
    pub fn box_tuple_position(it: *mut BoxTupleIterator) -> u32;

    /**
     * Rewind iterator to the initial position.
     *
     * \param it Tuple iterator
     * \post box_tuple_position(it) == 0
     */
    pub fn box_tuple_rewind(it: *mut BoxTupleIterator);

    /**
     * Seek the Tuple iterator.
     *
     * The returned buffer is valid until next call to box_tuple_* API.
     * Requested fieldno returned by next call to box_tuple_next(it).
     *
     * \param it Tuple iterator
     * \param fieldno - zero-based position in MsgPack array.
     * \post box_tuple_position(it) == fieldno if returned value is not NULL
     * \post box_tuple_position(it) == box_tuple_field_count(Tuple) if returned
     * value is NULL.
     */
    pub fn box_tuple_seek(it: *mut BoxTupleIterator, fieldno: u32) -> *const c_char;

    /**
     * Return the next Tuple field from Tuple iterator.
     * The returned buffer is valid until next call to box_tuple_* API.
     *
     * \param it Tuple iterator.
     * \retval NULL if there are no more fields.
     * \retval MsgPack otherwise
     * \pre box_tuple_position(it) is zerod-based id of returned field
     * \post box_tuple_position(it) == box_tuple_field_count(Tuple) if returned
     * value is NULL.
     */
    pub fn box_tuple_next(it: *mut BoxTupleIterator) -> *const c_char;

    pub fn box_tuple_update(tuple: *const BoxTuple, expr: *const c_char, expr_end: *const c_char) -> *mut BoxTuple;
    pub fn box_tuple_upsert(tuple: *const BoxTuple, expr: *const c_char, expr_end: *const c_char) -> *mut BoxTuple;
    pub fn box_tuple_extract_key(
        tuple: *const BoxTuple,
        space_id: u32,
        index_id: u32,
        key_size: *mut u32
    ) -> *mut c_char;
}

// ===========================================================================
// Space

pub const BOX_SYSTEM_ID_MIN: u32 = 256;
pub const BOX_SCHEMA_ID: u32 = 272;
pub const BOX_SPACE_ID: u32 = 280;
pub const BOX_VSPACE_ID: u32 = 281;
pub const BOX_INDEX_ID: u32 = 288;
pub const BOX_VINDEX_ID: u32 = 289;
pub const BOX_FUNC_ID: u32 = 296;
pub const BOX_VFUNC_ID: u32 = 297;
pub const BOX_USER_ID: u32 = 304;
pub const BOX_VUSER_ID: u32 = 305;
pub const BOX_PRIV_ID: u32 = 312;
pub const BOX_VPRIV_ID: u32 = 313;
pub const BOX_CLUSTER_ID: u32 = 320;
pub const BOX_SYSTEM_ID_MAX: u32 = 511;
pub const BOX_ID_NIL: u32 = 2147483647;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BoxFunctionCtx {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Return a Tuple from stored C procedure.
     *
     * Returned Tuple is automatically reference counted by Tarantool.
     *
     * \param ctx an opaque structure passed to the stored C procedure by
     * Tarantool
     * \param Tuple a Tuple to return
     * \retval -1 on error (perhaps, out of memory; check box_error_last())
     * \retval 0 otherwise
     */
    pub fn box_return_tuple(ctx: *mut BoxFunctionCtx, tuple: *mut BoxTuple) -> c_int;

    /**
     * Find space id by name.
     *
     * This function performs SELECT request to _vspace system space.
     * \param name space name
     * \param len length of \a name
     * \retval BOX_ID_NIL on error or if not found (check box_error_last())
     * \retval space_id otherwise
     * \sa box_index_id_by_name
     */
    pub fn box_space_id_by_name(name: *const c_char, len: u32) -> u32;

    /**
     * Find index id by name.
     *
     * This function performs SELECT request to _vindex system space.
     * \param space_id space identifier
     * \param name index name
     * \param len length of \a name
     * \retval BOX_ID_NIL on error or if not found (check box_error_last())
     * \retval index_id otherwise
     * \sa box_space_id_by_name
     */
    pub fn box_index_id_by_name(space_id: u32, name: *const c_char, len: u32) -> u32;

    /**
     * Execute an INSERT request.
     *
     * \param space_id space identifier
     * \param Tuple encoded Tuple in MsgPack Array format ([ field1, field2, ...])
     * \param tuple_end end of @a Tuple
     * \param[out] result a new Tuple. Can be set to NULL to discard result.
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id]:insert(Tuple) \endcode
     */
    pub fn box_insert(
        space_id: u32,
        tuple: *const c_char,
        tuple_end: *const c_char,
        result: *mut *mut BoxTuple
    ) -> c_int;

    /**
     * Execute an REPLACE request.
     *
     * \param space_id space identifier
     * \param Tuple encoded Tuple in MsgPack Array format ([ field1, field2, ...])
     * \param tuple_end end of @a Tuple
     * \param[out] result a new Tuple. Can be set to NULL to discard result.
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id]:replace(Tuple) \endcode
     */
    pub fn box_replace(
        space_id: u32,
        tuple: *const c_char,
        tuple_end: *const c_char,
        result: *mut *mut BoxTuple
    ) -> c_int;

    /**
     * Execute an DELETE request.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param key encoded key in MsgPack Array format ([part1, part2, ...]).
     * \param key_end the end of encoded \a key.
     * \param[out] result an old Tuple. Can be set to NULL to discard result.
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id].index[index_id]:delete(key) \endcode
     */
    pub fn box_delete(
        space_id: u32,
        index_id: u32,
        key: *const c_char,
        key_end: *const c_char,
        result: *mut *mut BoxTuple
    ) -> c_int;

    /**
     * Execute an UPDATE request.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param key encoded key in MsgPack Array format ([part1, part2, ...]).
     * \param key_end the end of encoded \a key.
     * \param ops encoded operations in MsgPack Arrat format, e.g.
     * [ [ '=', fieldno,  value ],  ['!', 2, 'xxx'] ]
     * \param ops_end the end of encoded \a ops
     * \param index_base 0 if fieldnos in update operations are zero-based
     * indexed (like C) or 1 if for one-based indexed field ids (like Lua).
     * \param[out] result a new Tuple. Can be set to NULL to discard result.
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id].index[index_id]:update(key, ops) \endcode
     * \sa box_upsert()
     */
    pub fn box_update(
        space_id: u32,
        index_id: u32,
        key: *const c_char,
        key_end: *const c_char,
        ops: *const c_char,
        ops_end: *const c_char,
        index_base: c_int,
        result: *mut *mut BoxTuple
    ) -> c_int;

    /**
     * Execute an UPSERT request.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param ops encoded operations in MsgPack Arrat format, e.g.
     * [ [ '=', fieldno,  value ],  ['!', 2, 'xxx'] ]
     * \param ops_end the end of encoded \a ops
     * \param Tuple encoded Tuple in MsgPack Array format ([ field1, field2, ...])
     * \param tuple_end end of @a Tuple
     * \param index_base 0 if fieldnos in update operations are zero-based
     * indexed (like C) or 1 if for one-based indexed field ids (like Lua).
     * \param[out] result a new Tuple. Can be set to NULL to discard result.
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id].index[index_id]:update(key, ops) \endcode
     * \sa box_update()
     */
    pub fn box_upsert(
        space_id: u32,
        index_id: u32,
        tuple: *const c_char,
        tuple_end: *const c_char,
        ops: *const c_char,
        ops_end: *const c_char,
        index_base: c_int,
        result: *mut *mut BoxTuple
    ) -> c_int;

    /**
     * Truncate space.
     *
     * \param space_id space identifier
     */
    pub fn box_truncate(space_id: u32) -> c_int;
}

// ===========================================================================
// Iterator

/** A space iterator */
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BoxIterator {
    _unused: [u8; 0],
}

/**
 * Controls how to iterate over tuples in an index.
 * Different index types support different iterator types.
 * For example, one can start iteration from a particular value
 * (request key) and then retrieve all tuples where keys are
 * greater or equal (= GE) to this key.
 *
 * If iterator type is not supported by the selected index type,
 * iterator constructor must fail with ER_UNSUPPORTED. To be
 * selectable for primary key, an index must support at least
 * ITER_EQ and ITER_GE types.
 *
 * NULL value of request key corresponds to the first or last
 * key in the index, depending on iteration direction.
 * (first key for GE and GT types, and last key for LE and LT).
 * Therefore, to iterate over all tuples in an index, one can
 * use ITER_GE or ITER_LE iteration types with start key equal
 * to NULL.
 * For ITER_EQ, the key must not be NULL.
 */
#[repr(i32)]
#[derive(Debug, Copy, Clone, ToPrimitive)]
pub enum IteratorType {
    /// key == x ASC order
    Eq = 0,

    /// key == x DESC order
    Req = 1,

    /// all tuples
    All = 2,

    /// key <  x
    LT = 3,

    /// key <= x
    LE = 4,

    /// key >= x
    GE = 5,

    /// key >  x
    GT = 6,

    /// all bits from x are set in key
    BitsAllSet = 7,

    /// at least one x's bit is set
    BitsAnySet = 8,

    /// all bits are not set
    BitsAllNotSet = 9,

    /// key overlaps x
    Ovelaps = 10,

    /// tuples in distance ascending order from specified point
    Neigbor = 11,
}

extern "C" {
    /**
     * Allocate and initialize iterator for space_id, index_id.
     *
     * A returned iterator must be destroyed by box_iterator_free().
     *
     * \param space_id space identifier.
     * \param index_id index identifier.
     * \param type \link iterator_type iterator type \endlink
     * \param key encoded key in MsgPack Array format ([part1, part2, ...]).
     * \param key_end the end of encoded \a key
     * \retval NULL on error (check box_error_last())
     * \retval iterator otherwise
     * \sa box_iterator_next()
     * \sa box_iterator_free()
     */
    pub fn box_index_iterator(
        space_id: u32,
        index_id: u32,
        type_: c_int,
        key: *const c_char,
        key_end: *const c_char
    ) -> *mut BoxIterator;

    /**
     * Retrive the next item from the \a iterator.
     *
     * \param iterator an iterator returned by box_index_iterator().
     * \param[out] result a Tuple or NULL if there is no more data.
     * \retval -1 on error (check box_error_last() for details)
     * \retval 0 on success. The end of data is not an error.
     */
    pub fn box_iterator_next(iterator: *mut BoxIterator, result: *mut *mut BoxTuple) -> c_int;

    /**
     * Destroy and deallocate iterator.
     *
     * \param iterator an interator returned by box_index_iterator()
     */
    pub fn box_iterator_free(iterator: *mut BoxIterator);

    /**
     * Return the number of element in the index.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \retval -1 on error (check box_error_last())
     * \retval >= 0 otherwise
     */
    pub fn box_index_len(space_id: u32, index_id: u32) -> isize;

    /**
     * Return the number of bytes used in memory by the index.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \retval -1 on error (check box_error_last())
     * \retval >= 0 otherwise
     */
    pub fn box_index_bsize(space_id: u32, index_id: u32) -> isize;

    /**
     * Return a random Tuple from the index (useful for statistical analysis).
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param rnd random seed
     * \param[out] result a Tuple or NULL if index is empty
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id].index[index_id]:random(rnd) \endcode
     */
    pub fn box_index_random(space_id: u32, index_id: u32, rnd: u32, result: *mut *mut BoxTuple) -> c_int;

    /**
     * Get a Tuple from index by the key.
     *
     * Please note that this function works much more faster than
     * box_select() or box_index_iterator() + box_iterator_next().
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param key encoded key in MsgPack Array format ([part1, part2, ...]).
     * \param key_end the end of encoded \a key
     * \param[out] result a Tuple or NULL if index is empty
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \pre key != NULL
     * \sa \code box.space[space_id].index[index_id]:get(key) \endcode
     */
    pub fn box_index_get(
        space_id: u32,
        index_id: u32,
        key: *const c_char,
        key_end: *const c_char,
        result: *mut *mut BoxTuple
    ) -> c_int;

    /**
     * Return a first (minimal) Tuple matched the provided key.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param key encoded key in MsgPack Array format ([part1, part2, ...]).
     * \param key_end the end of encoded \a key.
     * \param[out] result a Tuple or NULL if index is empty
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id].index[index_id]:min(key) \endcode
     */
    pub fn box_index_min(
        space_id: u32,
        index_id: u32,
        key: *const c_char,
        key_end: *const c_char,
        result: *mut *mut BoxTuple) -> c_int;

    /**
     * Return a last (maximal) Tuple matched the provided key.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param key encoded key in MsgPack Array format ([part1, part2, ...]).
     * \param key_end the end of encoded \a key.
     * \param[out] result a Tuple or NULL if index is empty
     * \retval -1 on error (check box_error_last())
     * \retval 0 on success
     * \sa \code box.space[space_id].index[index_id]:max(key) \endcode
     */
    pub fn box_index_max(
        space_id: u32,
        index_id: u32,
        key: *const c_char,
        key_end: *const c_char,
        result: *mut *mut BoxTuple
    ) -> c_int;

    /**
     * Count the number of Tuple matched the provided key.
     *
     * \param space_id space identifier
     * \param index_id index identifier
     * \param type iterator type - enum \link iterator_type \endlink
     * \param key encoded key in MsgPack Array format ([part1, part2, ...]).
     * \param key_end the end of encoded \a key.
     * \retval -1 on error (check box_error_last())
     * \retval >=0 on success
     * \sa \code box.space[space_id].index[index_id]:count(key,
     *     { iterator = type }) \endcode
     */
    pub fn box_index_count(
        space_id: u32,
        index_id: u32,
        type_: c_int,
        key: *const c_char,
        key_end: *const c_char
    ) -> isize;
}

// ===========================================================================
// Error

/**
 * Error - contains information about error.
 */
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BoxError {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Return the error type, e.g. "ClientError", "SocketError", etc.
     * \param error
     * \return not-null string
     */
    pub fn box_error_type(error: *const BoxError) -> *const c_char;

    /**
     * Return IPROTO error code
     * \param error error
     * \return enum box_error_code
     */
    pub fn box_error_code(error: *const BoxError) -> u32;

    /**
     * Return the error message
     * \param error error
     * \return not-null string
     */
    pub fn box_error_message(error: *const BoxError) -> *const c_char;

    /**
     * Get the information about the last API call error.
     *
     * The Tarantool error handling works most like libc's errno. All API calls
     * return -1 or NULL in the event of error. An internal pointer to
     * BoxError type is set by API functions to indicate what went wrong.
     * This value is only significant if API call failed (returned -1 or NULL).
     *
     * Successful function can also touch the last error in some
     * cases. You don't have to clear the last error before calling
     * API functions. The returned object is valid only until next
     * call to **any** API function.
     *
     * You must set the last error using box_error_set() in your stored C
     * procedures if you want to return a custom error message.
     * You can re-throw the last API error to IPROTO client by keeping
     * the current value and returning -1 to Tarantool from your
     * stored procedure.
     *
     * \return last error.
     */
    pub fn box_error_last() -> *mut BoxError;

    /**
     * Clear the last error.
     */
    pub fn box_error_clear();

    /**
     * Set the last error.
     *
     * \param code IPROTO error code (enum \link box_error_code \endlink)
     * \param format (const char * ) - printf()-like format string
     * \param ... - format arguments
     * \returns -1 for convention use
     *
     * \sa enum box_error_code
     */
    pub fn box_error_set(file: *const c_char, line: c_uint, code: u32, format: *const c_char, ...)-> c_int;
}

// ===========================================================================
// Latch

/**
 * A lock for cooperative multitasking environment
 */
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BoxLatch {
    _unused: [u8; 0],
}

extern "C" {
    /**
     * Allocate and initialize the new latch.
     * \returns latch
     */
    pub fn box_latch_new() -> *mut BoxLatch;

    /**
     * Destroy and free the latch.
     * \param latch latch
     */
    pub fn box_latch_delete(latch: *mut BoxLatch);

    /**
     * Lock a latch. Waits indefinitely until the current fiber can gain access to
     * the latch.
     *
     * \param latch a latch
     */
    pub fn box_latch_lock(latch: *mut BoxLatch);

    /**
     * Try to lock a latch. Return immediately if the latch is locked.
     * \param latch a latch
     * \retval 0 - success
     * \retval 1 - the latch is locked.
     */
    pub fn box_latch_trylock(latch: *mut BoxLatch) -> c_int;

    /**
     * Unlock a latch. The fiber calling this function must
     * own the latch.
     *
     * \param latch a latch
     */
    pub fn box_latch_unlock(latch: *mut BoxLatch);
}

// ===========================================================================
// Clock

extern "C" {
    pub fn clock_realtime() -> f64;
    pub fn clock_monotonic() -> f64;
    pub fn clock_process() -> f64;
    pub fn clock_thread() -> f64;
    pub fn clock_realtime64() -> u64;
    pub fn clock_monotonic64() -> u64;
    pub fn clock_process64() -> u64;
    pub fn clock_thread64() -> u64;
}
