use tarantool::{
    tlua::LuaFunction,
    tuple::Tuple,
    uuid::Uuid,
};

const UUID_STR: &str = "30de7784-33e2-4393-a8cd-b67534db2432";

pub fn to_tuple() {
    let u = Uuid::parse_str(UUID_STR).unwrap();
    let t = Tuple::from_struct(&(u,)).unwrap();
    let lua = tarantool::global_lua();
    let f: LuaFunction<_> = lua.eval("return box.tuple.unpack").unwrap();
    let u: Uuid = f.call_with_args(t).unwrap();
    assert_eq!(u.to_string(), UUID_STR);
}

pub fn from_tuple() {
    let lua = tarantool::global_lua();
    let t: Tuple = lua.eval(
        &format!("return box.tuple.new(require('uuid').fromstr('{}'))", UUID_STR)
    ).unwrap();
    let (u,): (Uuid,) = t.as_struct().unwrap();
    assert_eq!(u.to_string(), UUID_STR);
}

pub fn to_lua() {
    let lua = tarantool::global_lua();
    let uuid: Uuid = lua.eval(
        &format!("return require('uuid').fromstr('{}')", UUID_STR)
    ).unwrap();
    assert_eq!(uuid.to_string(), UUID_STR);
}

pub fn from_lua() {
    let uuid = Uuid::parse_str(UUID_STR).unwrap();
    let lua = tarantool::global_lua();
    let tostring: LuaFunction<_> = lua.eval("return tostring").unwrap();
    let s: String = tostring.call_with_args(uuid).unwrap();
    assert_eq!(s, UUID_STR);
}

