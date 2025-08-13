use crate::JsonValue;

#[macro_export]
macro_rules! json {
    (null) => { $crate::JsonValue::Null };
    (true) => { $crate::JsonValue::Bool(true) };
    (false) => { $crate::JsonValue::Bool(false) };

    ([ $($elems:tt),* $(,)? ]) => {{
        let mut v = Vec::new();
        $( v.push($crate::json!($elems)); )*
        $crate::JsonValue::Array(v)
    }};

    ({ $($key:tt : $value:tt),* $(,)? }) => {{
        let mut m = ::std::collections::HashMap::new();
        $( m.insert(::std::string::ToString::to_string(&$crate::__json_key!($key)), $crate::json!($value)); )*
        $crate::JsonValue::Object(m)
    }};

    ($other:expr) => {{
        $crate::__json_from_expr($other)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __json_key {
    ($key:literal) => {
        $key
    };
    ($key:expr) => {
        $key
    };
}

#[doc(hidden)]
#[inline]
pub fn __json_from_expr<T>(value: T) -> JsonValue
where
    T: __IntoJson,
{
    value.__into_json()
}

#[doc(hidden)]
pub trait __IntoJson {
    fn __into_json(self) -> JsonValue;
}

impl __IntoJson for JsonValue {
    #[inline]
    fn __into_json(self) -> JsonValue {
        self
    }
}

impl __IntoJson for &str {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::String(self.to_string())
    }
}
impl __IntoJson for String {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::String(self)
    }
}
impl __IntoJson for bool {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::Bool(self)
    }
}
impl __IntoJson for i64 {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::Number(self as f64)
    }
}
impl __IntoJson for i32 {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::Number(self as f64)
    }
}
impl __IntoJson for u64 {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::Number(self as f64)
    }
}
impl __IntoJson for u32 {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::Number(self as f64)
    }
}
impl __IntoJson for f64 {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::Number(self)
    }
}
impl __IntoJson for f32 {
    #[inline]
    fn __into_json(self) -> JsonValue {
        JsonValue::Number(self as f64)
    }
}

impl<T> __IntoJson for Vec<T>
where
    T: __IntoJson,
{
    fn __into_json(self) -> JsonValue {
        JsonValue::Array(self.into_iter().map(|v| v.__into_json()).collect())
    }
}

impl<T, const N: usize> __IntoJson for [T; N]
where
    T: __IntoJson,
{
    fn __into_json(self) -> JsonValue {
        let mut vec = Vec::with_capacity(N);
        for item in self {
            vec.push(item.__into_json());
        }
        JsonValue::Array(vec)
    }
}
