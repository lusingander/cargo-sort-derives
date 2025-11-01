#[cfg_attr(test, derive(Default, Debug, Clone))]
pub struct C1 {
    c: char,
}

#[cfg_attr(all(feature = "serde", not(test)), derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct C2 {
    c: char,
}
