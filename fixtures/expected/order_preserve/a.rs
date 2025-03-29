#[derive(Default, Debug, Eq, PartialEq, Clone, Hash, Copy)]
pub struct A1 {
    a: i32,
}

mod a {
    // sort-derives-disable-next-line
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct A2 {
        a: i32,
    }

    // ...
    // sort-derives-disable-next-line
    // ...
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct A3 {
        a: i32,
    }
}
