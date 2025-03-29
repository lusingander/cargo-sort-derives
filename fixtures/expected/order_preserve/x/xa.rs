use std::cmp;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Xa1 {
    s: String,
}

#[derive(std::fmt::Debug, Clone, std::cmp::PartialEq, cmp::Eq)]
struct Xa2 {
    i: i32,
}

// mod xa3 {
//     #[derive(Clone, Copy, PartialEq, Eq)]
//     pub struct Xa3 {
//         f: f32,
//     }
// }
