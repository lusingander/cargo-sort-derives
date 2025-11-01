#[cfg_attr(test, derive(Debug, Clone, Default))]
pub struct C1 {
    c: char,
}

#[cfg_attr(all(feature = "serde", not(test)), derive(serde::Serialize, serde::Deserialize, Debug))]
pub struct C2 {
    c: char,
}
