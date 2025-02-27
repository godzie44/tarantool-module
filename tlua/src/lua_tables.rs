use std::convert::From;
use std::marker::PhantomData;
use std::num::NonZeroI32;

use crate::{
    ffi,
    AbsoluteIndex,
    AsLua,
    Push,
    PushInto,
    PushGuard,
    PushOne,
    PushOneInto,
    LuaError,
    LuaFunction,
    LuaRead,
    LuaState,
    LuaFunctionCallError,
    Void,
};

/// Represents a table stored in the Lua context.
///
/// Just like you can read variables as integers and strings, you can also read Lua table by
/// requesting a `LuaTable` object. Doing so will mutably borrow the object which you got the table
/// from.
///
/// # Example: reading a global variable
///
/// ```
/// let mut lua = tlua::Lua::new();
/// lua.exec("a = {28, 92, 17};").unwrap();
///
/// let mut table: tlua::LuaTable<_> = lua.get("a").unwrap();
/// for (k, v) in table.iter::<i32, i32>().filter_map(|e| e) {
///     println!("{} => {}", k, v);
/// }
/// ```
///
#[derive(Debug)]
pub struct LuaTable<L> {
    lua: L,
    index: AbsoluteIndex,
}

impl<L> LuaTable<L>
where
    L: AsLua,
{
    fn new(lua: L, index: NonZeroI32) -> Self {
        Self {
            index: AbsoluteIndex::new(index, lua.as_lua()),
            lua,
        }
    }
}

impl<L> AsLua for LuaTable<L>
where
    L: AsLua,
{
    #[inline]
    fn as_lua(&self) -> LuaState {
        self.lua.as_lua()
    }
}

impl<L> LuaRead<L> for LuaTable<L>
where
    L: AsLua,
{
    #[inline]
    fn lua_read_at_position(lua: L, index: NonZeroI32) -> Result<LuaTable<L>, L> {
        if unsafe { ffi::lua_istable(lua.as_lua(), index.into()) } {
            Ok(LuaTable::new(lua, index))
        } else {
            Err(lua)
        }
    }
}

impl<L, T> Push<L> for LuaTable<T>
where
    L: AsLua,
{
    type Err = Void;

    fn push_to_lua(&self, lua: L) -> Result<PushGuard<L>, (Void, L)> {
        unsafe {
            ffi::lua_pushvalue(lua.as_lua(), self.index.into());
            Ok(PushGuard::new(lua, 1))
        }
    }
}

impl<L, T> PushOne<L> for LuaTable<T>
where
    L: AsLua,
{
}

impl<'lua, L> LuaTable<L>
where
    L: 'lua,
    L: AsLua,
{
    /// Destroys the `LuaTable` and returns its inner Lua context. Useful when it takes Lua by
    /// value.
    // TODO: find an example where it is useful
    #[inline]
    pub fn into_inner(self) -> L {
        self.lua
    }

    /// Iterates over the elements inside the table.
    // TODO: doc
    #[inline]
    pub fn iter<K, V>(&self) -> LuaTableIterator<L, K, V> {
        unsafe {
            ffi::lua_pushnil(self.lua.as_lua());

            LuaTableIterator {
                table: self,
                finished: false,
                last_top: ffi::lua_gettop(self.lua.as_lua()),
                marker: PhantomData,
            }
        }
    }

    /// Loads a value in the table given its index.
    ///
    /// The index must implement the `PushOne` trait and the return type must implement the
    /// `LuaRead` trait. See
    /// [the documentation at the crate root](index.html#pushing-and-loading-values) for more
    /// information.
    ///
    /// # Example: reading a table inside of a table.
    ///
    /// ```
    /// let mut lua = tlua::Lua::new();
    /// lua.exec("a = { 9, { 8, 7 }, 6 }").unwrap();
    ///
    /// let mut table = lua.get::<tlua::LuaTable<_>, _>("a").unwrap();
    ///
    /// assert_eq!(table.get::<i32, _, _>(1).unwrap(), 9);
    /// assert_eq!(table.get::<i32, _, _>(3).unwrap(), 6);
    ///
    /// {
    ///     let mut subtable: tlua::LuaTable<_> = table.get(2).unwrap();
    ///     assert_eq!(subtable.get::<i32, _, _>(1).unwrap(), 8);
    ///     assert_eq!(subtable.get::<i32, _, _>(2).unwrap(), 7);
    /// }
    /// ```
    ///
    #[inline]
    pub fn get<R, I>(&'lua self, index: I) -> Option<R>
    where
        I: PushOneInto<LuaState, Err = Void>,
        R: LuaRead<PushGuard<&'lua L>>,
    {
        Self::get_impl(&self.lua, self.index.into(), index).ok()
    }

    /// Loads a value in the table, with the result capturing the table by value.
    // TODO: doc
    #[inline]
    pub fn into_get<R, I>(self, index: I) -> Result<R, PushGuard<Self>>
    where
        I: PushOneInto<LuaState, Err = Void>,
        R: LuaRead<PushGuard<Self>>,
    {
        let this_index = self.index.into();
        Self::get_impl(self, this_index, index)
    }

    #[inline(always)]
    pub fn get_impl<T, R, I>(
        this: T,
        this_index: i32,
        index: I,
    ) -> Result<R, PushGuard<T>>
    where
        T: AsLua,
        I: PushOneInto<LuaState, Err = Void>,
        R: LuaRead<PushGuard<T>>,
    {
        let raw_lua = this.as_lua();
        unsafe {
            index.push_into_no_err(raw_lua).assert_one_and_forget();
            ffi::lua_gettable(raw_lua, this_index);
            R::lua_read(PushGuard::new(this, 1))
        }
    }

    /// Inserts or modifies an elements of the table.
    ///
    /// Contrary to `checked_set`, can only be called when writing the key and value cannot fail
    /// (which is the case for most types).
    ///
    /// The index and the value must both implement the `PushOne` trait. See
    /// [the documentation at the crate root](index.html#pushing-and-loading-values) for more
    /// information.
    // TODO: doc
    #[inline(always)]
    pub fn set<I, V, Ei, Ev>(&self, index: I, value: V)
    where
        I: PushOneInto<LuaState, Err = Ei>,
        V: PushOneInto<LuaState, Err = Ev>,
        Ei: Into<Void>,
        Ev: Into<Void>,
    {
        match self.checked_set(index, value) {
            Ok(()) => (),
            Err(_) => unreachable!(),
        }
    }

    /// Inserts or modifies an elements of the table.
    ///
    /// Returns an error if we failed to write the key and the value. This can only happen for a
    /// limited set of types. You are encouraged to use the `set` method if writing cannot fail.
    // TODO: doc
    #[inline]
    pub fn checked_set<I, V>(
        &self,
        index: I,
        value: V,
    ) -> Result<(), CheckedSetError<<I as PushInto<LuaState>>::Err, <V as PushInto<LuaState>>::Err>>
    where
        I: PushOneInto<LuaState>,
        V: PushOneInto<LuaState>,
    {
        unsafe {
            let guard = match index.push_into_lua(self.as_lua()) {
                Ok(guard) => {
                    assert_eq!(guard.size, 1);
                    guard
                }
                Err((err, _)) => {
                    return Err(CheckedSetError::KeyPushError(err));
                }
            };

            match value.push_into_lua(self.as_lua()) {
                Ok(pushed) => {
                    assert_eq!(pushed.size, 1);
                    pushed.forget()
                }
                Err((err, _)) => {
                    return Err(CheckedSetError::ValuePushError(err));
                }
            };

            guard.forget();
            ffi::lua_settable(self.as_lua(), self.index.into());
            Ok(())
        }
    }

    pub fn call_method<R, A>(&'lua self, name: &str, args: A)
        -> Result<R, MethodCallError<<A as Push<LuaState>>::Err>>
    where
        L: std::fmt::Debug,
        A: Push<LuaState>,
        A: std::fmt::Debug,
        R: LuaRead<PushGuard<LuaFunction<PushGuard<&'lua L>>>>,
    {
        let method: LuaFunction<_> = self.get(name).ok_or(MethodCallError::NoSuchMethod)?;
        method.into_call_with_args((self, args))
            .map_err(|e|
                match e {
                    LuaFunctionCallError::LuaError(e) => MethodCallError::LuaError(e),
                    LuaFunctionCallError::PushError(e) => {
                        MethodCallError::PushError(e.other().first())
                    }
                }
            )
    }

    /// Inserts an empty array, then loads it.
    #[inline]
    pub fn empty_array<I>(&'lua self, index: I) -> LuaTable<PushGuard<&'lua L>>
    where
        I: PushOne<LuaState, Err = Void>,
    {
        unsafe {
            self.as_lua().push(&index).assert_one_and_forget();
            ffi::lua_newtable(self.as_lua());
            ffi::lua_settable(self.as_lua(), self.index.into());
            self.get(&index).unwrap()
        }
    }

    /// Obtains or creates the metatable of the table.
    ///
    /// A metatable is an additional table that can be attached to a table or a userdata. It can
    /// contain anything, but its most interesting usage are the following special methods:
    ///
    /// - If non-nil, the `__index` entry of the metatable is used as a function whenever the user
    ///   tries to read a non-existing entry in the table or userdata. Its signature is
    ///   `(object, index) -> value`.
    /// - If non-nil, the `__newindex` entry of the metatable is used as a function whenever the
    ///   user tries to write a non-existing entry in the table or userdata. Its signature is
    ///   `(object, index, value)`.
    /// - If non-nil, the `__lt`, `__le` and `__eq` entries correspond respectively to operators
    ///    `<`, `<=` and `==`. Their signature is `(a, b) -> bool`. Other operators are
    ///   automatically derived from these three functions.
    /// - If non-nil, the `__add`, `__mul`, `__sub`, `__div`, `__unm`, `__pow` and `__concat`
    ///   entries correspond to operators `+`, `*`, `-`, `/`, `-` (unary), `^` and `..`. Their
    ///   signature is `(a, b) -> result`.
    /// - If non-nil, the `__gc` entry is called whenever the garbage collector is about to drop
    ///   the object. Its signature is simply `(obj)`. Remember that usercode is able to modify
    ///   the metatable as well, so there is no strong guarantee that this is actually going to be
    ///   called.
    ///
    /// Interestingly enough, a metatable can also have a metatable. For example if you try to
    /// access a non-existing field in a table, Lua will look for the `__index` function in its
    /// metatable. If that function doesn't exist, it will try to use the `__index` function of the
    /// metatable's metatable in order to get the `__index` function of the metatable. This can
    /// go on infinitely.
    ///
    /// # Example
    ///
    /// ```
    /// use tlua::Lua;
    /// use tlua::LuaTable;
    /// use tlua::AnyLuaValue;
    ///
    /// let mut lua = Lua::new();
    /// lua.exec("a = {}").unwrap();
    ///
    /// {
    ///     let mut table: LuaTable<_> = lua.get("a").unwrap();
    ///     let mut metatable = table.get_or_create_metatable();
    ///     metatable.set("__index", tlua::function2(|_: AnyLuaValue, var: String| -> AnyLuaValue {
    ///         println!("The user tried to access non-existing index {:?}", var);
    ///         AnyLuaValue::LuaNil
    ///     }));
    /// }
    /// ```
    #[inline]
    pub fn get_or_create_metatable(self) -> LuaTable<PushGuard<L>> {
        unsafe {
            // We put the metatable at the top of the stack.
            if ffi::lua_getmetatable(self.lua.as_lua(), self.index.into()) == 0 {
                // No existing metatable ; create one then set it and reload it.
                ffi::lua_newtable(self.lua.as_lua());
                ffi::lua_setmetatable(self.lua.as_lua(), self.index.into());
                let r = ffi::lua_getmetatable(self.lua.as_lua(), self.index.into());
                debug_assert!(r != 0);
            }

            LuaTable::new(PushGuard::new(self.lua, 1), crate::NEGATIVE_ONE)
        }
    }

    /// Builds the `LuaTable` that yields access to the registry.
    ///
    /// The registry is a special table available from anywhere and that is not directly
    /// accessible from Lua code. It can be used to store whatever you want to keep in memory.
    ///
    /// # Example
    ///
    /// ```
    /// use tlua::Lua;
    /// use tlua::LuaTable;
    ///
    /// let mut lua = Lua::new();
    ///
    /// let mut table = LuaTable::registry(&mut lua);
    /// table.set(3, "hello");
    /// ```
    #[inline]
    pub fn registry(lua: L) -> LuaTable<L> {
        LuaTable::new(
            lua,
            unsafe { NonZeroI32::new_unchecked(ffi::LUA_REGISTRYINDEX) }
        )
    }
}

#[derive(Debug)]
pub enum MethodCallError<E> {
    NoSuchMethod,
    LuaError(LuaError),
    PushError(E),
}

impl<E> From<LuaFunctionCallError<E>> for MethodCallError<E> {
    fn from(e: LuaFunctionCallError<E>) -> Self {
        match e {
            LuaFunctionCallError::PushError(e) => MethodCallError::PushError(e),
            LuaFunctionCallError::LuaError(e) => MethodCallError::LuaError(e),
        }
    }
}

/// Error returned by the `checked_set` function.
// TODO: implement `Error` on this type
#[derive(Debug, Copy, Clone)]
pub enum CheckedSetError<K, V> {
    /// Error while pushing the key.
    KeyPushError(K),
    /// Error while pushing the value.
    ValuePushError(V),
}

/// Iterator that enumerates the content of a Lua table.
///
/// See `LuaTable::iter` for more info.
// Implementation note: While the LuaTableIterator is active, the current key is constantly
// pushed over the table. The destructor takes care of removing it.
#[derive(Debug)]
pub struct LuaTableIterator<'t, L: 't, K, V>
where
    L: AsLua,
{
    table: &'t LuaTable<L>,
    finished: bool, // if true, the key is not on the stack anymore
    last_top: i32,
    marker: PhantomData<(K, V)>,
}

impl<'t, L, K, V> Iterator for LuaTableIterator<'t, L, K, V>
where
    L: AsLua + 't,
    K: LuaRead<&'t LuaTable<L>>,
    V: LuaRead<PushGuard<&'t LuaTable<L>>>,
{
    type Item = Option<(K, V)>;

    #[inline]
    fn next(&mut self) -> Option<Option<(K, V)>> {
        unsafe {
            if self.finished {
                return None;
            }

            // The key must always be at the top of the stack unless
            // `finished` is true. Because the `value` may capture the pushguard
            // by value and the caller will be responsibe for dropping the stack
            // values, we need to make sure the stack is in the correct
            // configuration before invoking `lua_next`.
            assert_eq!(self.last_top, ffi::lua_gettop(self.table.as_lua()),
                "lua stack is corrupt"
            );
            // This call pops the current key and pushes the next key and value at the top.
            if ffi::lua_next(self.table.as_lua(), self.table.index.into()) == 0 {
                self.finished = true;
                return None;
            }

            // The key must remain on the stack, but the value must be dropped
            // before next iteration. If `V` captures the `guard`, the user
            // must make sure it is dropped before calling `next` on this
            // iterator, otherwise it will result in a panic
            let guard = PushGuard::new(self.table, 1);

            // Reading the key and value.
            let key = K::lua_read_at_position(self.table, crate::NEGATIVE_TWO).ok();
            let value = V::lua_read_at_position(guard, crate::NEGATIVE_ONE).ok();

            if key.is_none() || value.is_none() {
                Some(None)
            } else {
                Some(Some((key.unwrap(), value.unwrap())))
            }
        }
    }
}

impl<'t, L, K, V> Drop for LuaTableIterator<'t, L, K, V>
where
    L: AsLua,
{
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if !self.finished {
                ffi::lua_pop(self.table.as_lua(), 1);
            }
        }
    }
}

