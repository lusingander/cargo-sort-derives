// Multi-line plain derive (rustfmt style)
#[derive(
    Hash,
    Clone,
    Debug,
)]
pub struct Multi1;

// Multi-line cfg_attr with nested condition
#[cfg_attr(all(feature = "serde", not(test)), derive(Deserialize, Serialize, Debug))]
pub struct Multi2;

// sort-derives-disable-next-line
#[derive(
    Hash,
    Clone,
    Debug,
)]
pub struct Multi3;
