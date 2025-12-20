use sandl::*;

#[test]
fn from_value_i64() {
    let v = Value::from(42i64);
    assert_eq!(i64::from_value(&v).unwrap(), 42);
}

#[test]
fn from_value_array() {
    let v = Value::Array(vec![Value::from(1), Value::from(2), Value::from(3)]);
    let arr: [i32; 3] = FromValue::from_value(&v).unwrap();
    assert_eq!(arr, [1, 2, 3]);
}

#[test]
fn from_value_option() {
    let v = Value::Null;
    let opt: Option<i32> = FromValue::from_value(&v).unwrap();
    assert_eq!(opt, None);

    let v = Value::from(42);
    let opt: Option<i32> = FromValue::from_value(&v).unwrap();
    assert_eq!(opt, Some(42));
}

#[test]
fn to_value_array() {
    let arr = [1, 2, 3];
    let v = arr.to_value();
    assert_eq!(v.as_array().unwrap().len(), 3);
}
