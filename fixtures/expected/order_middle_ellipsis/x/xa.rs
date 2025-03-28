use std::cmp;

#[derive(Eq, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Xa1 {
    s: String,
}

#[derive(cmp::Eq, Clone, std::fmt::Debug, std::cmp::PartialEq)]
struct Xa2 {
    i: i32,
}

// mod xa3 {
//     #[derive(Eq, Clone, Copy, PartialEq)]
//     pub struct Xa3 {
//         f: f32,
//     }
// }
