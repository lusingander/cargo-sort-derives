#[derive(Clone, Debug)]
pub struct B1 {
    b: bool,
}

// sort-derives-disable-start

#[derive(Debug, Clone)]
pub struct B2 {
    b: bool,
}

#[derive(Debug, Clone)]
pub struct B3 {
    b: bool,
}

// sort-derives-disable-end

#[derive(Clone, Debug)]
pub struct B4 {
    b: bool,
}
