use std::cmp;

#[derive(Clone, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
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
