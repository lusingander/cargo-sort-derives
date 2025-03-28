use std::cmp;

#[derive(Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct Xa1 {
    s: String,
}

#[derive(Clone, std::fmt::Debug, cmp::Eq, std::cmp::PartialEq)]
struct Xa2 {
    i: i32,
}

// mod xa3 {
//     #[derive(Clone, Copy, Eq, PartialEq)]
//     pub struct Xa3 {
//         f: f32,
//     }
// }
