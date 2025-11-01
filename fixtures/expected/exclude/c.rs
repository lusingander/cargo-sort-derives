#[cfg_attr(test, derive(Clone, Debug, Default))]
pub struct C1 {
    c: char,
}

#[cfg_attr(all(feature = "serde", not(test)), derive(Debug, serde::Deserialize, serde::Serialize))]
pub struct C2 {
    c: char,
}
