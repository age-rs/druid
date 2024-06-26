// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Test #[derive(Data)]

use druid::Data;

#[derive(Data, Clone)]
struct PlainStruct;

#[derive(Data, Clone)]
struct EmptyTupleStruct();

#[derive(Data, Clone)]

struct SingleTupleStruct(bool);

#[derive(Data, Clone)]
struct MultiTupleStruct(bool, i64, String);

#[derive(Data, Clone)]
struct EmptyFieldStruct {}

#[derive(Data, Clone)]
struct SingleFieldStruct {
    a: bool,
}

#[derive(Data, Clone)]
struct MultiFieldStruct {
    a: bool,
    b: i64,
    c: String,
}

trait UserTrait {}

#[derive(Clone, Data)]
struct TypeParamForUserTraitStruct<T: UserTrait + Data> {
    a: T,
}

#[derive(Clone, Data)]
struct TypeParamForUserTraitWithWhereClauseStruct<T>
where
    T: UserTrait,
{
    b: T,
}

#[derive(Clone, Data)]
enum TypeParamForUserTraitAndLifetimeEnum<T: UserTrait + 'static> {
    V1(T),
}

#[test]
fn test_data_derive_same() {
    let plain = PlainStruct;
    assert!(plain.same(&plain));

    let empty_tuple = EmptyTupleStruct();
    assert!(empty_tuple.same(&empty_tuple));

    let singletuple = SingleTupleStruct(true);
    assert!(singletuple.same(&singletuple));
    assert!(!singletuple.same(&SingleTupleStruct(false)));

    let multituple = MultiTupleStruct(false, 33, "Test".to_string());
    assert!(multituple.same(&multituple));
    assert!(!multituple.same(&MultiTupleStruct(true, 33, "Test".to_string())));

    let empty_field = EmptyFieldStruct {};
    assert!(empty_field.same(&empty_field));

    let singlefield = SingleFieldStruct { a: true };
    assert!(singlefield.same(&singlefield));
    assert!(!singlefield.same(&SingleFieldStruct { a: false }));

    let multifield = MultiFieldStruct {
        a: false,
        b: 33,
        c: "Test".to_string(),
    };
    assert!(multifield.same(&multifield));
    assert!(!multifield.same(&MultiFieldStruct {
        a: false,
        b: 33,
        c: "Fail".to_string()
    }));

    #[derive(Clone, Data)]
    struct Value(u32);

    impl UserTrait for Value {}

    let v = TypeParamForUserTraitStruct { a: Value(1) };
    assert!(v.same(&v));
    assert!(!v.same(&TypeParamForUserTraitStruct { a: Value(2) }));

    let v = TypeParamForUserTraitWithWhereClauseStruct { b: Value(3) };
    assert!(v.same(&v));
    assert!(!v.same(&TypeParamForUserTraitWithWhereClauseStruct { b: Value(6) }));

    let v = TypeParamForUserTraitAndLifetimeEnum::V1(Value(10));
    assert!(v.same(&v));
    assert!(!v.same(&TypeParamForUserTraitAndLifetimeEnum::V1(Value(12))));
}

#[derive(Data, Clone)]
struct DataAttrEq {
    #[data(eq)]
    f: PanicOnPartialEq,
}

#[derive(Clone, Copy)]
struct PanicOnPartialEq;
impl PartialEq for PanicOnPartialEq {
    fn eq(&self, _other: &Self) -> bool {
        panic!("PartialEq::eq called");
    }
}

#[test]
#[should_panic = "PartialEq::eq called"]
fn data_attr_eq() {
    DataAttrEq {
        f: PanicOnPartialEq,
    }
    .same(&DataAttrEq {
        f: PanicOnPartialEq,
    });
}
